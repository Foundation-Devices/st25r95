// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(not(test), no_std)]

mod command;
mod control;
mod error;
mod protocol;
mod register;

use {
    crate::{command::Command, control::Control},
    acc_a::{AccA, DemodulatorSensitivity, LoadModulationIndex},
    arc_b::{ArcB, ModulationIndex, ReceiverGain},
    auto_detect_filter::AutoDetectFilter,
    command::{CtrlResConf, DacData, LFOFreq, WaitForField, WakeUpSource},
    core::{fmt::Debug, marker::PhantomData, str::from_utf8},
    embedded_hal::{
        delay::DelayNs,
        digital::{InputPin, OutputPin},
        spi::SpiDevice,
    },
    iso14443a::{
        card_emulation::{AntiColState, Listen},
        ATQA,
        SAK,
        UID,
    },
    iso15693::reader::Modulation,
    timer_window::TimerWindow,
    wakeup::Wakeup,
};
pub use {
    crate::{command::IdleParams, control::PollFlags, protocol::*, register::*},
    error::{Error, Result},
};

// Type State Field
#[derive(Debug, Default)]
pub struct FieldOn;
#[derive(Debug, Default)]
pub struct FieldOff;

// Type State Role
#[derive(Debug)]
pub struct Reader;
#[derive(Debug)]
pub struct CardEmulation(Listen);
#[derive(Debug)]
pub struct NoRole;

// Type State Protocol
#[derive(Debug)]
pub struct Iso15693(Modulation);
#[derive(Debug)]
pub struct Iso14443A;
#[derive(Debug)]
pub struct Iso14443B;
#[derive(Debug)]
pub struct FeliCa;
#[derive(Debug)]
pub struct NoProtocol;

type ResultSt25r95FieldOff<'a, SPI, D, I, O, R, P> =
    Result<St25r95<'a, SPI, D, I, O, FieldOff, R, P>, SPI, I, O>;
type ResultSt25r95ReaderIso15693<'a, SPI, D, I, O> =
    Result<St25r95<'a, SPI, D, I, O, FieldOn, Reader, Iso15693>, SPI, I, O>;
type ResultSt25r95ReaderIso14443A<'a, SPI, D, I, O> =
    Result<St25r95<'a, SPI, D, I, O, FieldOn, Reader, Iso14443A>, SPI, I, O>;
type ResultSt25r95ReaderIso14443B<'a, SPI, D, I, O> =
    Result<St25r95<'a, SPI, D, I, O, FieldOn, Reader, Iso14443B>, SPI, I, O>;
type ResultSt25r95ReaderFelica<'a, SPI, D, I, O> =
    Result<St25r95<'a, SPI, D, I, O, FieldOn, Reader, FeliCa>, SPI, I, O>;
type ResultSt25r95CardEmulationIso14443A<'a, SPI, D, I, O> =
    Result<St25r95<'a, SPI, D, I, O, FieldOn, CardEmulation, Iso14443A>, SPI, I, O>;

pub const MAX_BUFFER_SIZE: usize = 530;

pub struct St25r95<'a, SPI, D, I, O, F, R, P> {
    spi: SPI,
    delay: D,
    irq_out: I,
    irq_in: O,
    buf: &'a mut [u8],
    dac_ref: Option<u8>,
    dac_guard: u8,
    field: PhantomData<F>,
    role: R,
    protocol: P,
}

impl<'a, SPI: SpiDevice, D: DelayNs, I: InputPin, O: OutputPin>
    St25r95<'a, SPI, D, I, O, FieldOff, NoRole, NoProtocol>
{
    pub fn new(
        spi: SPI,
        delay: D,
        irq_out: I,
        irq_in: O,
        buf: &'a mut [u8],
    ) -> Result<Self, SPI, I, O> {
        // do not assume any state
        let mut st25r95 = Self {
            spi,
            delay,
            irq_out,
            irq_in,
            buf,
            dac_ref: None,
            dac_guard: 0,
            field: PhantomData,
            role: NoRole,
            protocol: NoProtocol,
        };
        // Startup sequence §3.2
        st25r95.irq_in_pulse_low()?;
        // should be in Ready state
        st25r95.reset()?;
        let (idn_str, _) = st25r95.idn()?;
        if !idn_str.starts_with("NFC") {
            return Err(Error::IdentificationError);
        }
        Ok(st25r95)
    }

    /// The Echo command verifies the possibility of communication between a Host and the
    /// ST25R95.
    pub fn echo(&mut self) -> Result<(), SPI, I, O> {
        self.poll_spi(PollFlags::CAN_SEND)?;

        self.buf[0] = Control::Send as u8;
        self.buf[1] = Command::Echo as u8;
        self.spi.write(&self.buf[..2]).map_err(Error::Spi)?;

        self.poll_irq_out(100)?;

        self.buf[0] = Control::Read as u8;
        self.buf[1] = 0;
        self.spi
            .transfer_in_place(&mut self.buf[..2])
            .map_err(Error::Spi)?;
        if self.buf[0] == Command::Echo as u8 {
            Ok(())
        } else {
            //TODO: flush Chip SPI buffer
            Err(Error::EchoFailed)
        }
    }
}

impl<'a, SPI: SpiDevice, D: DelayNs, I: InputPin, O: OutputPin, R: Default, P: Default>
    St25r95<'a, SPI, D, I, O, FieldOn, R, P>
{
    pub fn field_off(mut self) -> ResultSt25r95FieldOff<'a, SPI, D, I, O, R, P> {
        self.select_protocol(Protocol::FieldOff, protocol::FieldOff)?;
        Ok(St25r95 {
            spi: self.spi,
            delay: self.delay,
            irq_out: self.irq_out,
            irq_in: self.irq_in,
            buf: self.buf,
            dac_ref: self.dac_ref,
            dac_guard: self.dac_guard,
            field: PhantomData::<FieldOff>,
            role: R::default(),
            protocol: P::default(),
        })
    }
}

impl<'a, SPI: SpiDevice, D: DelayNs, I: InputPin, O: OutputPin, F, R, P>
    St25r95<'a, SPI, D, I, O, F, R, P>
{
    fn irq_in_pulse_low(&mut self) -> Result<(), SPI, I, O> {
        self.irq_in.set_high().map_err(Error::IrqIn)?;
        self.delay.delay_ms(1); // t0 > 100us
        self.irq_in.set_low().map_err(Error::IrqIn)?;
        self.delay.delay_ms(1); // t1 > 10us
        self.irq_in.set_high().map_err(Error::IrqIn)?;
        self.delay.delay_ms(11); // t3 > 10ms

        // should be in Ready state
        Ok(())
    }

    fn reset(&mut self) -> Result<(), SPI, I, O> {
        self.spi
            .write(&[Control::Reset as u8])
            .map_err(Error::Spi)?;
        self.delay.delay_ms(3);
        // should be in Power-up state
        self.irq_in_pulse_low()?;
        Ok(())
    }

    fn send_command(&mut self, cmd: Command, data: &[u8]) -> Result<(), SPI, I, O> {
        if data.len() > self.buf.len() - 3 {
            return Err(Error::InternalBufferOverflow);
        }

        self.buf[0] = Control::Send as u8;
        self.buf[1] = cmd as u8;
        self.buf[2] = data.len() as u8;
        self.buf[3..3 + data.len()].copy_from_slice(data);

        self.spi
            .write(&self.buf[..3 + data.len()])
            .map_err(Error::Spi)
    }

    fn poll_irq_out(&mut self, timeout: impl Into<Option<u32>> + Copy) -> Result<(), SPI, I, O> {
        let mut curr_timeout = 0u32;
        loop {
            if self.irq_out.is_low().map_err(Error::IrqOut)? {
                return Ok(());
            }

            if let Some(timeout) = timeout.into() {
                if curr_timeout > timeout {
                    //TODO: flush Chip SPI buffer
                    return Err(Error::PollTimeout);
                }

                curr_timeout += 1;
            }
            self.delay.delay_ms(1);
        }
    }

    fn poll_spi(&mut self, flags: PollFlags) -> Result<(), SPI, I, O> {
        let mut curr_flags = [0; 2];
        self.spi
            .transfer(&mut curr_flags, &[Control::Poll as u8, Control::Poll as u8])
            .map_err(Error::Spi)?;
        match PollFlags::from_bits_truncate(curr_flags[1]).contains(flags) {
            true => Ok(()),
            false => Err(Error::PollTimeout),
        }
    }

    fn read(&mut self) -> Result<ReadResponse, SPI, I, O> {
        self.poll_irq_out(100)?;

        self.buf[0] = Control::Read as u8;
        self.spi
            .transfer_in_place(&mut self.buf[..2])
            .map_err(Error::Spi)?;

        //TODO: how to keep CS low after read header

        let response = ReadResponse::new(&self.buf[1..3]);
        if response.len != 0 {
            if response.len as usize > self.buf.len() {
                //TODO: flush Chip SPI buffer
                return Err(Error::InvalidResponseLength {
                    expected: self.buf.len() as u16,
                    actual: response,
                });
            }

            self.spi
                .read(&mut self.buf[..response.len as usize])
                .map_err(Error::Spi)?;
        }

        if response.code == 0 || response.code == 0x80 || response.code == 0x90 {
            Ok(response)
        } else {
            Err(Error::Hw(response.code.into()))
        }
    }

    /// The IDN command gives brief information about the ST25R95 and its revision.
    pub fn idn(&mut self) -> Result<(&str, u16), SPI, I, O> {
        self.send_command(Command::Idn, &[])?;

        let response = self.read()?;
        if response.len != 15 {
            return Err(Error::InvalidResponseLength {
                expected: 15,
                actual: response,
            });
        }

        let resp = &self.buf[..response.len.into()];

        let idn_str = from_utf8(&resp[..13])?;
        let rom_crc = ((resp[13] as u16) << 8) | resp[14] as u16; // §4.1.1 SPI communication is MSB first.
        Ok((idn_str, rom_crc))
    }

    fn select_protocol(
        &mut self,
        protocol: Protocol,
        params: impl ProtocolParams,
    ) -> Result<(), SPI, I, O> {
        let mut data = [0u8; 9];
        data[0] = protocol as u8;
        let (d, data_len) = params.data();
        if data_len > 0 {
            data[1..1 + data_len].copy_from_slice(&d[..data_len]);
        }

        self.send_command(Command::ProtocolSelect, &data[..1 + data_len])?;

        let response = self.read()?;
        if response.len != 0 {
            Err(Error::InvalidResponseLength {
                expected: 0,
                actual: response,
            })
        } else {
            Ok(())
        }
    }

    /// This command selects the RF communication protocol and prepares the ST25R95 for
    /// communication with contactless ISO/IEC 15693 tag.
    pub fn protocol_select_iso15693(
        mut self,
        params: iso15693::reader::Parameters,
    ) -> ResultSt25r95ReaderIso15693<'a, SPI, D, I, O> {
        let modulation = params.get_modulation();
        self.select_protocol(Protocol::Iso15693, params)?;
        Ok(St25r95 {
            spi: self.spi,
            delay: self.delay,
            irq_out: self.irq_out,
            irq_in: self.irq_in,
            buf: self.buf,
            dac_ref: self.dac_ref,
            dac_guard: self.dac_guard,
            field: PhantomData::<FieldOn>,
            role: Reader,
            protocol: Iso15693(modulation),
        })
    }

    /// This command selects the RF communication protocol and prepares the ST25R95 for
    /// communication with contactless ISO/IEC 14443-A tag.
    pub fn protocol_select_iso14443a(
        mut self,
        params: iso14443a::reader::Parameters,
    ) -> ResultSt25r95ReaderIso14443A<'a, SPI, D, I, O> {
        self.select_protocol(Protocol::Iso14443A, params)?;
        Ok(St25r95 {
            spi: self.spi,
            delay: self.delay,
            irq_out: self.irq_out,
            irq_in: self.irq_in,
            buf: self.buf,
            dac_ref: self.dac_ref,
            dac_guard: self.dac_guard,
            field: PhantomData::<FieldOn>,
            role: Reader,
            protocol: Iso14443A,
        })
    }

    /// This command selects the RF communication protocol and prepares the ST25R95 for
    /// communication with contactless ISO/IEC 14443-B tag.
    pub fn protocol_select_iso14443b(
        mut self,
        params: iso14443b::reader::Parameters,
    ) -> ResultSt25r95ReaderIso14443B<'a, SPI, D, I, O> {
        self.select_protocol(Protocol::Iso14443B, params)?;
        Ok(St25r95 {
            spi: self.spi,
            delay: self.delay,
            irq_out: self.irq_out,
            irq_in: self.irq_in,
            buf: self.buf,
            dac_ref: self.dac_ref,
            dac_guard: self.dac_guard,
            field: PhantomData::<FieldOn>,
            role: Reader,
            protocol: Iso14443B,
        })
    }

    /// This command selects the RF communication protocol and prepares the ST25R95 for
    /// communication with contactless FeliCa tag.
    pub fn protocol_select_felica(
        mut self,
        params: felica::reader::Parameters,
    ) -> ResultSt25r95ReaderFelica<'a, SPI, D, I, O> {
        self.select_protocol(Protocol::FeliCa, params)?;
        Ok(St25r95 {
            spi: self.spi,
            delay: self.delay,
            irq_out: self.irq_out,
            irq_in: self.irq_in,
            buf: self.buf,
            dac_ref: self.dac_ref,
            dac_guard: self.dac_guard,
            field: PhantomData::<FieldOn>,
            role: Reader,
            protocol: FeliCa,
        })
    }

    /// This command selects the RF communication protocol and prepares the ST25R95 for
    /// communication with a reader in Card Emulation with ISO/IEC 14443-A.
    pub fn protocol_select_ce_iso14443a(
        mut self,
        params: iso14443a::card_emulation::Parameters,
    ) -> ResultSt25r95CardEmulationIso14443A<'a, SPI, D, I, O> {
        self.select_protocol(Protocol::CardEmulationIso14443A, params)?;
        Ok(St25r95 {
            spi: self.spi,
            delay: self.delay,
            irq_out: self.irq_out,
            irq_in: self.irq_in,
            buf: self.buf,
            dac_ref: self.dac_ref,
            dac_guard: self.dac_guard,
            field: PhantomData::<FieldOn>,
            role: CardEmulation(false),
            protocol: Iso14443A,
        })
    }

    /// This command can be used to detect the presence/absence of an HF field by
    /// monitoring the field detector (FieldDet) flag. It can be used as well to wait for
    /// HF field appearance or disappearance until a defined timeout expires. The answer
    /// to the PollField command is the value of the FieldDet flag.
    /// The result of this command depends on the protocol selected. If a reader mode
    /// protocol is selected, the flag FieldDet is set to ‘1’ because the RF field is
    /// turned ON by the reader.
    pub fn poll_field(&mut self, wff: Option<WaitForField>) -> Result<bool, SPI, I, O> {
        match wff {
            None => self.send_command(Command::PollField, &[])?,
            Some(WaitForField {
                apparance,
                presc,
                timer,
            }) => self.send_command(Command::PollField, &[apparance as u8, presc, timer])?,
        }

        let response = self.read()?;
        match response.len {
            0 => Ok(false),
            1 => Ok(self.buf[0] & 0x01 == 1),
            _ => Err(Error::InvalidResponseLength {
                expected: 1,
                actual: response,
            }),
        }
    }

    fn _idle(
        &mut self,
        mut params: IdleParams,
        check_params: bool,
    ) -> Result<WakeUpSource, SPI, I, O> {
        if check_params && params.wus.tag_detection {
            match self.dac_ref {
                None => return Err(Error::CalibrationNeeded),
                Some(dac_ref) => {
                    params.dac_data.high =
                        dac_ref
                            .checked_add(self.dac_guard)
                            .ok_or(Error::TagDetector {
                                dac_ref,
                                dac_guard: self.dac_guard,
                            })?;
                    params.dac_data.low =
                        dac_ref
                            .checked_sub(self.dac_guard)
                            .ok_or(Error::TagDetector {
                                dac_ref,
                                dac_guard: self.dac_guard,
                            })?;
                }
            }
        }
        self.send_command(Command::Idle, &params.data())?;

        let response = self.read()?;
        if response.len != 1 {
            Err(Error::InvalidResponseLength {
                expected: 1,
                actual: response,
            })
        } else {
            self.buf[0]
                .try_into()
                .map_err(|_| Error::InvalidWakeUpSource(self.buf[0]))
            // should be in WaitForEvent state
        }
    }

    /// This command switches the ST25R95 into low power consumption mode and defines the
    /// way to return to Ready state.
    ///
    /// Caution:
    /// In low power consumption mode the device does not support SPI poll mechanism.
    /// Application has to rely on IRQ_OUT before reading the answer to the Idle command.
    pub fn idle(&mut self, params: IdleParams) -> Result<WakeUpSource, SPI, I, O> {
        self._idle(params, true)
    }

    fn _write_register(
        &mut self,
        reg: &impl Register,
        inc_addr: bool,
        value: Option<u8>,
    ) -> Result<(), SPI, I, O> {
        let mut data = [0u8; 4];
        data[0] = reg.write_addr();
        data[1] = inc_addr as u8;
        if reg.has_index() {
            data[2] = reg.index_confirmation();
        } else {
            data[3] = reg.index_confirmation();
        }
        let data_len = if let Some(value) = value {
            if reg.has_index() {
                data[3] = value;
            } else {
                data[2] = value;
            }
            4
        } else {
            3
        };
        self.send_command(Command::WrReg, &data[..data_len])?;

        let response = self.read()?;
        if response.len != 0 {
            Err(Error::InvalidResponseLength {
                expected: 0,
                actual: response,
            })
        } else {
            Ok(())
        }
    }

    fn read_register(&mut self, reg: &impl Register) -> Result<u8, SPI, I, O> {
        if reg.has_index() {
            // Set register index first
            self._write_register(reg, false, None)?;
        }
        let mut data = [0u8; 3];
        data[0] = reg.read_addr();
        data[1] = 0x01;
        data[2] = 0x00;
        self.send_command(Command::RdReg, &data)?;

        let response = self.read()?;
        if response.len != 1 {
            Err(Error::InvalidResponseLength {
                expected: 1,
                actual: response,
            })
        } else {
            Ok(self.buf[0])
        }
    }

    /// This command is used to read the Wakeup register.
    pub fn wakeup_source(&mut self) -> Result<WakeUpSource, SPI, I, O> {
        let reg = Wakeup;
        let value = self.read_register(&reg)?;
        value
            .try_into()
            .map_err(|_| Error::InvalidWakeUpSource(value))
    }
}

impl<'a, SPI: SpiDevice, D: DelayNs, I: InputPin, O: OutputPin, P: Default>
    St25r95<'a, SPI, D, I, O, FieldOn, Reader, P>
{
    /// This command sends data to a contactless tag and receives its reply.
    /// If the tag response was received and decoded correctly, the <Data> field can
    /// contain additional information which is protocol-specific.
    pub fn send_receive(&mut self, data: &[u8]) -> Result<(u8, &[u8]), SPI, I, O> {
        self.send_command(Command::SendRecv, data)?;

        let response = self.read()?;
        Ok((response.code, &self.buf[..response.len as usize]))
    }

    /// Calibrate the tag detector as wake-up source by an iterrative process.
    pub fn calibrate_tag_detector(&mut self) -> Result<(), SPI, I, O> {
        let mut params = IdleParams {
            wus: WakeUpSource {
                lfo_freq: LFOFreq::KHz32,
                ss_low_pulse: false,
                irq_in_low_pulse: false,
                field_detection: false,
                tag_detection: true,
                timeout: true,
            },
            enter_ctrl: CtrlResConf {
                field_detector_enabled: false,
                iref_enabled: false,
                dac_comp_high: true,
                lfo_enabled: true,
                hfo_enabled: false,
                vdda_enabled: false,
                hibernate_state_enabled: false,
                sleep_state_enabled: true,
            },
            wu_ctrl: CtrlResConf {
                field_detector_enabled: false,
                iref_enabled: true,
                dac_comp_high: true,
                lfo_enabled: true,
                hfo_enabled: true,
                vdda_enabled: true,
                hibernate_state_enabled: false,
                sleep_state_enabled: false,
            },
            wu_period: 0,
            dac_data: DacData {
                low: 0x00,
                high: 0x00,
            },
            max_sleep: 0x01,
            ..Default::default()
        };
        let wus = self._idle(params, false)?;
        if !wus.tag_detection {
            return Err(Error::CalibTagDetectionFailed);
        }
        params.dac_data.high = 0xFC; // max value
        let mut wus = self._idle(params, false)?;
        if !wus.timeout {
            return Err(Error::CalibTimeoutFailed);
        }
        for &val in [0x80, 0x40, 0x20, 0x10, 0x08, 0x04].iter() {
            if wus.timeout {
                params.dac_data.high -= val;
            } else if wus.tag_detection {
                params.dac_data.high += val;
            }
            wus = self._idle(params, false)?;
        }
        if wus.timeout {
            params.dac_data.high -= 0x04;
        }
        self.dac_ref = Some(params.dac_data.high);
        Ok(())
    }

    /// This command is used to read the ARC_B register.
    pub fn read_arc_b(&mut self) -> Result<ArcB, SPI, I, O> {
        ArcB::from_u8(self.read_register(&ArcB::fake())?)
    }

    pub fn write_arc_b(&mut self, arc_b: ArcB) -> Result<(), SPI, I, O> {
        self._write_register(&arc_b, false, Some(arc_b.value()))
    }

    /// The Echo command verifies the possibility of communication between a Host and the
    /// ST25R95.
    pub fn echo(&mut self) -> Result<(), SPI, I, O> {
        self.poll_spi(PollFlags::CAN_SEND)?;

        self.buf[0] = Control::Send as u8;
        self.buf[1] = Command::Echo as u8;
        self.spi.write(&self.buf[..2]).map_err(Error::Spi)?;

        self.poll_irq_out(100)?;

        self.buf[0] = Control::Read as u8;
        self.buf[1] = 0;
        self.spi
            .transfer_in_place(&mut self.buf[..2])
            .map_err(Error::Spi)?;
        if self.buf[0] == Command::Echo as u8 {
            Ok(())
        } else {
            //TODO: flush Chip SPI buffer
            Err(Error::EchoFailed)
        }
    }
}

impl<'a, SPI: SpiDevice, D: DelayNs, I: InputPin, O: OutputPin>
    St25r95<'a, SPI, D, I, O, FieldOn, Reader, Iso15693>
{
    pub fn new_arc_b(
        &self,
        modulation_index: ModulationIndex,
        receiver_gain: ReceiverGain,
    ) -> Result<ArcB, SPI, I, O> {
        // See Table 35
        if match self.protocol.0 {
            Modulation::Percent10 => [
                ModulationIndex::Percent30,
                ModulationIndex::Percent33,
                ModulationIndex::Percent36,
            ]
            .contains(&modulation_index),
            Modulation::Percent100 => [ModulationIndex::Percent95].contains(&modulation_index),
        } {
            Ok(ArcB {
                modulation_index,
                receiver_gain,
            })
        } else {
            Err(Error::InvalidModulationIndex(modulation_index as u8))
        }
    }

    pub fn default_arc_b(&self) -> ArcB {
        // See Table 35
        self.new_arc_b(
            match self.protocol.0 {
                Modulation::Percent10 => ModulationIndex::Percent33,
                Modulation::Percent100 => ModulationIndex::Percent95,
            },
            ReceiverGain::Db27,
        )
        .unwrap()
    }
}

impl<'a, SPI: SpiDevice, D: DelayNs, I: InputPin, O: OutputPin>
    St25r95<'a, SPI, D, I, O, FieldOn, Reader, Iso14443A>
{
    pub fn new_arc_b(
        &self,
        modulation_index: ModulationIndex,
        receiver_gain: ReceiverGain,
    ) -> Result<ArcB, SPI, I, O> {
        // See Table 35
        if [ModulationIndex::Percent95].contains(&modulation_index) {
            Ok(ArcB {
                modulation_index,
                receiver_gain,
            })
        } else {
            Err(Error::InvalidModulationIndex(modulation_index as u8))
        }
    }

    pub fn default_arc_b(&self) -> ArcB {
        // See Table 35
        self.new_arc_b(ModulationIndex::Percent95, ReceiverGain::Db8)
            .unwrap()
    }

    pub fn new_timer_window(&self, timer_w: u8) -> Result<TimerWindow, SPI, I, O> {
        if (0x50..=0x60).contains(&timer_w) {
            Ok(TimerWindow(timer_w))
        } else {
            Err(Error::InvalidU8Parameter {
                min: 0x50,
                max: 0x60,
                actual: timer_w,
            })
        }
    }

    pub fn default_timer_window(&self) -> TimerWindow {
        // See §5.11.2
        self.new_timer_window(0x52).unwrap()
    }

    pub fn recommended_timer_window(&self) -> TimerWindow {
        // See §5.11.2
        self.new_timer_window(0x56).unwrap()
    }

    /// To improve ST25R95 demodulation when communicating with ISO/IEC 14443 Type A tags,
    /// it is possible to adjust the synchronization between digital and analog inputs
    /// by fine-tuning the Timer Window value.
    /// The default values of these parameters are set by the ProtocolSelect command, but
    /// they can be overwritten using this function.
    pub fn write_timer_windows(&mut self, timer_w: TimerWindow) -> Result<(), SPI, I, O> {
        self._write_register(&timer_w, false, Some(timer_w.value()))
    }
}

impl<'a, SPI: SpiDevice, D: DelayNs, I: InputPin, O: OutputPin>
    St25r95<'a, SPI, D, I, O, FieldOn, Reader, Iso14443B>
{
    pub fn new_arc_b(
        &self,
        modulation_index: ModulationIndex,
        receiver_gain: ReceiverGain,
    ) -> Result<ArcB, SPI, I, O> {
        // See Table 35
        if [
            ModulationIndex::Percent10,
            ModulationIndex::Percent17,
            ModulationIndex::Percent25,
            ModulationIndex::Percent30,
        ]
        .contains(&modulation_index)
        {
            Ok(ArcB {
                modulation_index,
                receiver_gain,
            })
        } else {
            Err(Error::InvalidModulationIndex(modulation_index as u8))
        }
    }

    pub fn default_arc_b(&self) -> ArcB {
        // See Table 35
        self.new_arc_b(ModulationIndex::Percent17, ReceiverGain::Db34)
            .unwrap()
    }
}

impl<'a, SPI: SpiDevice, D: DelayNs, I: InputPin, O: OutputPin>
    St25r95<'a, SPI, D, I, O, FieldOn, Reader, FeliCa>
{
    pub fn new_arc_b(
        &self,
        modulation_index: ModulationIndex,
        receiver_gain: ReceiverGain,
    ) -> Result<ArcB, SPI, I, O> {
        // See Table 35
        if [
            ModulationIndex::Percent10,
            ModulationIndex::Percent17,
            ModulationIndex::Percent25,
            ModulationIndex::Percent30,
        ]
        .contains(&modulation_index)
        {
            Ok(ArcB {
                modulation_index,
                receiver_gain,
            })
        } else {
            Err(Error::InvalidModulationIndex(modulation_index as u8))
        }
    }

    pub fn default_arc_b(&self) -> ArcB {
        // See Table 35
        self.new_arc_b(ModulationIndex::Percent33, ReceiverGain::Db34)
            .unwrap()
    }

    /// To improve ST25R95 reception when communicating with FeliCa™ tags, it is possible
    /// to enable an AutoDetect filter to synchronize FeliCa™ tags with the ST25R95.
    /// By default, this filter is disabled after the execution of the ProtocolSelect
    /// command, but it can be enabled using this function.
    pub fn enable_autodetect_filter(&mut self) -> Result<(), SPI, I, O> {
        let reg = AutoDetectFilter;
        self._write_register(&reg, false, Some(reg.value()))
    }
}

impl<'a, SPI: SpiDevice, D: DelayNs, I: InputPin, O: OutputPin>
    St25r95<'a, SPI, D, I, O, FieldOn, CardEmulation, Iso14443A>
{
    /// In card emulation mode, this function puts the ST25R95 in Listening mode.
    /// The ST25R95 will exit Listening mode as soon it receives the Echo command from the
    /// Host Controller (MCU) or a command from an external reader (not including commands
    /// supported by the AC filter command).
    /// If no command from an external reader has been received, then the Echo command
    /// must be used to exit the Listening mode prior to sending a new command to the
    /// ST25R95.
    pub fn listen(&mut self) -> Result<(), SPI, I, O> {
        self.send_command(Command::Listen, &[])?;

        let response = self.read()?;
        if response.len != 0 {
            Err(Error::InvalidResponseLength {
                expected: 0,
                actual: response,
            })
        } else {
            self.role.0 = true;
            Ok(())
        }
    }

    /// The Echo command verifies the possibility of communication between a Host and the
    /// ST25R95.
    /// The ST25R95 will exit the listen mode upon reception of an echo command.
    /// This can be used to stop listen mode.
    pub fn echo(&mut self) -> Result<(), SPI, I, O> {
        self.poll_spi(PollFlags::CAN_SEND)?;

        self.buf[0] = Control::Send as u8;
        self.buf[1] = Command::Echo as u8;
        self.spi.write(&self.buf[..2]).map_err(Error::Spi)?;

        self.poll_irq_out(100)?;

        self.buf[0] = Control::Read as u8;
        self.buf[1] = 0;
        self.buf[2] = 0;
        self.buf[3] = 0;
        self.spi
            .transfer_in_place(&mut self.buf[..if self.role.0 { 4 } else { 2 }])
            .map_err(Error::Spi)?;
        if self.buf[0] != Command::Echo as u8 {
            //TODO: flush Chip SPI buffer
            return Err(Error::EchoFailed);
        }
        if self.role.0 {
            let response = ReadResponse::new(&self.buf[1..3]);
            if response.code != 0x85 {
                return Err(Error::EchoFailed);
            }
            if response.len != 0 {
                return Err(Error::InvalidResponseLength {
                    expected: 0,
                    actual: response,
                });
            }
            self.role.0 = false; // Listening mode was cancelled by the application
        }
        Ok(())
    }

    /// Receive data from the reader through the ST25R95 in Listen mode.
    /// Will be blocking until data is available.
    pub fn receive(&mut self) -> Result<(u8, &[u8]), SPI, I, O> {
        let response = self.read()?;
        Ok((response.code, &self.buf[..response.len as usize]))
    }

    /// Immediately sends data to the reader using the Load Modulation method.
    pub fn send(&mut self, data: &[u8]) -> Result<(), SPI, I, O> {
        self.send_command(Command::Send, data)?;

        let response = self.read()?;
        if response.len != 0 {
            Err(Error::InvalidResponseLength {
                expected: 0,
                actual: response,
            })
        } else {
            Ok(())
        }
    }

    pub fn new_acc_a(
        &self,
        load_modulation_index: LoadModulationIndex,
        demodulator_sensitivity: DemodulatorSensitivity,
    ) -> Result<AccA, SPI, I, O> {
        // See Table 36
        if demodulator_sensitivity != DemodulatorSensitivity::Percent100 {
            Err(Error::InvalidDemodulatorSensitivity(
                demodulator_sensitivity as u8,
            ))
        } else {
            Ok(AccA {
                load_modulation_index,
                demodulator_sensitivity,
            })
        }
    }

    pub fn default_acc_a(&self) -> AccA {
        self.new_acc_a(
            LoadModulationIndex::default(),
            DemodulatorSensitivity::Percent100,
        )
        .unwrap()
    }

    pub fn recommended_acc_a(&self) -> AccA {
        self.default_acc_a()
    }

    /// This command is used to read the ACC_A register.
    pub fn read_acc_a(&mut self) -> Result<AccA, SPI, I, O> {
        AccA::from_u8(self.read_register(&self.default_acc_a())?)
    }

    /// Adjusting the Load modulation index and Demodulator sensitivity parameters in card
    /// emulation mode can help to improve application behavior.
    /// The default values of these parameters are set by the ProtocolSelect command, but
    /// they can be overwritten using this function.
    pub fn write_acc_a(&mut self, acc_a: AccA) -> Result<(), SPI, I, O> {
        self._write_register(&acc_a, false, Some(acc_a.value()))
    }

    /// This command activates the anti-collision filter in Type A card emulation mode.
    ///
    /// ## Parameters
    /// - cascade_level_filter: 1 to 3 UIDs, other number will return
    ///   InvalidCascadeLevelFilterCount
    pub fn activate_ac_filter(
        &mut self,
        atqa: ATQA,
        sak: SAK,
        cascade_level_filter: impl IntoIterator<Item = UID>,
    ) -> Result<(), SPI, I, O> {
        let mut clf_len = 0;
        let mut data = [0u8; 15];
        data[0..2].copy_from_slice(&atqa.to_le_bytes());
        data[2] = sak;
        for uid in cascade_level_filter.into_iter() {
            if clf_len > 3 {
                return Err(Error::InvalidCascadeLevelFilterCount(clf_len));
            }
            data[3 + clf_len..3 + clf_len + uid.len()].copy_from_slice(uid.as_slice());
            clf_len += 1;
        }
        if clf_len == 0 {
            return Err(Error::InvalidCascadeLevelFilterCount(clf_len));
        }
        self.send_command(Command::ACFilter, &data[..3 + clf_len])?;

        let response = self.read()?;
        if response.len != 0 {
            Err(Error::InvalidResponseLength {
                expected: 0,
                actual: response,
            })
        } else {
            Ok(())
        }
    }

    fn ac_filter_state(&mut self, data: &[u8]) -> Result<AntiColState, SPI, I, O> {
        self.send_command(Command::ACFilter, data)?;

        let response = self.read()?;
        if response.len != 1 {
            Err(Error::InvalidResponseLength {
                expected: 1,
                actual: response,
            })
        } else {
            AntiColState::try_from(self.buf[0]).map_err(|_| Error::InvalidAntiColState(self.buf[0]))
        }
    }

    /// This command de-activates the anti-collision filter in Type A card emulation mode.
    pub fn deactivate_ac_filter(&mut self) -> Result<AntiColState, SPI, I, O> {
        self.ac_filter_state(&[])
    }

    /// This command read the Anti-Collision Filter state in Type A card emulation mode.
    /// Does not de-activate the filter.
    pub fn anti_collision_state(&mut self) -> Result<AntiColState, SPI, I, O> {
        self.ac_filter_state(&[0x00, 0x00])
    }

    /// This command sets the Anti-Collision Filter state in Type A card emulation mode.
    pub fn set_anti_collision_state(&mut self, state: AntiColState) -> Result<(), SPI, I, O> {
        self.send_command(Command::ACFilter, &[state as u8])?;

        let response = self.read()?;
        if response.len != 0 {
            Err(Error::InvalidResponseLength {
                expected: 0,
                actual: response,
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ReadResponse {
    pub code: u8,
    pub len: u16,
}

impl ReadResponse {
    pub fn new(header: &[u8]) -> Self {
        assert_eq!(header.len(), 2);
        // See datasheet section 4.3 (Support of long frames)
        Self {
            code: header[0],
            len: header[1] as u16
                | if header[0] & 0x80 == 0x80 {
                    (header[0] as u16 & 0b0110_0000) << 3
                } else {
                    0
                },
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, core::ops::Range};

    #[test]
    pub fn test_len_decode() {
        // See datasheet Table 8.
        check_range(0x80, 0x00..0xff, 0..255);
        check_range(0xA0, 0x00..0xff, 256..511);
        check_range(0xC0, 0x00..0x10, 512..528);
        check_range(0x90, 0x00..0xff, 0..255);
        check_range(0xB0, 0x00..0xff, 256..511);
        check_range(0xD0, 0x00..0x10, 512..528);
    }

    fn check_range(code: u8, len_range: Range<u8>, res_range: Range<u16>) {
        for len in len_range {
            let res = ReadResponse::new(&[code, len]).len;
            assert!(res_range.contains(&res))
        }
    }
}
