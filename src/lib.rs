// SPDX-FileCopyrightText: 2023 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(not(test), no_std)]

mod callbacks;
mod command;
mod control;
mod error;
mod protocol;
mod register;

use {
    crate::{command::Command, control::Control, error::St25r95Error::SpiError},
    acc_a::AccA,
    arc_b::ArcB,
    command::{CtrlResConf, DacData, IdleParams, LFOFreq, WaitForField, WakeUpSource},
    core::{fmt::Debug, str::from_utf8},
    iso15693::Modulation,
};

pub use crate::{
    callbacks::Callbacks,
    control::PollFlags,
    error::St25r95Error,
    protocol::*,
    register::*,
};

pub struct St25r95<'a, E: Debug, C: Callbacks<Error = E>> {
    cb: C,
    buf: &'a mut [u8],
    protocol: Option<Protocol>,
    modulation: Option<Modulation>,
    dac_ref: Option<u8>,
    dac_guard: u8,
    listen_mode: bool,
}

const TIMING_T0: u8 = 1;
const TIMING_T1: u8 = 1;
const TIMING_T3: u8 = 10;

impl<'a, E: Debug, C: Callbacks<Error = E>> St25r95<'a, E, C> {
    pub fn new(cb: C, buf: &'a mut [u8]) -> Self {
        Self {
            cb,
            buf,
            protocol: None,
            modulation: None,
            dac_ref: None,
            dac_guard: 0x08,
            listen_mode: false,
        }
    }

    pub fn init(&mut self) -> Result<(), St25r95Error<E>> {
        self.reset()?;
        let (idn_str, _) = self.idn()?;
        if !idn_str.starts_with("NFC") {
            return Err(St25r95Error::IdentificationError);
        }
        Ok(())
    }

    fn irq_pulse(&self) {
        self.cb.set_irq_in(true);
        self.cb.delay_ms(TIMING_T0);
        self.cb.set_irq_in(false);
        self.cb.delay_ms(TIMING_T1);
        self.cb.set_irq_in(true);
        self.cb.delay_ms(TIMING_T3);
    }

    fn reset(&mut self) -> Result<(), St25r95Error<E>> {
        self.irq_pulse();
        self.cb.select();
        self.send_control(Control::Reset)?;
        self.cb.release();
        Ok(())
    }

    fn send_control(&mut self, control: Control) -> Result<u8, St25r95Error<E>> {
        let mut dummy = [0];
        self.cb
            .transfer(&[control as u8], &mut dummy)
            .map_err(|e| SpiError(e))?;
        Ok(dummy[0])
    }

    fn send_command(&mut self, cmd: Command, data: &[u8]) -> Result<(), St25r95Error<E>> {
        if data.len() >= self.buf.len() {
            return Err(St25r95Error::InternalBufferOverflow);
        }

        self.irq_pulse();

        // Clear the dummy buffer for the data that'll go over the SPI bus
        let dummy = &mut self.buf[..data.len()];
        dummy.fill(0);

        self.cb.select();

        self.send_control(Control::Command)?;

        // Send command header
        let header_dummy = &mut self.buf[..2];
        self.cb
            .transfer(&[cmd as u8, data.len() as u8], header_dummy)
            .map_err(|e| SpiError(e))?;
        header_dummy.fill(0);

        // Send command data
        let data_dummy = &mut self.buf[..data.len()];
        self.cb
            .transfer(data, data_dummy)
            .map_err(|e| SpiError(e))?;

        self.cb.release();

        Ok(())
    }

    pub fn poll(
        &mut self,
        timeout: impl Into<Option<u32>> + Copy,
        flags: PollFlags,
    ) -> Result<(), St25r95Error<E>> {
        let mut curr_timeout = 0u32;

        self.irq_pulse();
        self.cb.select();

        loop {
            let curr_flags = PollFlags::from_bits_truncate(self.send_control(Control::Poll)?);
            if curr_flags.contains(flags) {
                self.cb.release();
                return Ok(());
            }

            if let Some(timeout) = timeout.into() {
                if curr_timeout > timeout {
                    return Err(St25r95Error::PollTimeout);
                }

                curr_timeout += 1;
            }
            // TODO? missing a 1ms delay here to be sure we poll only once every 1ms
            // self.cb.delay_ms(1);
        }
    }

    fn read(&mut self) -> Result<ReadResponse, St25r95Error<E>> {
        self.irq_pulse();
        self.send_control(Control::Read)?;

        let mut response_header_buf = [0u8; 2];
        self.cb
            .read(response_header_buf.as_mut_slice())
            .map_err(|e| SpiError(e))?;

        let response = ReadResponse::new(&response_header_buf);
        if response.len != 0 {
            if response.len as usize > self.buf.len() {
                return Err(St25r95Error::InvalidResponseLength {
                    expected: self.buf.len() as u16,
                    actual: response.len,
                });
            }

            self.cb
                .read(&mut self.buf[..response.len as usize])
                .map_err(|e| SpiError(e))?;
        }
        self.cb.release();

        if response.code == 0 || response.code == 0x80 || response.code == 0x90 {
            Ok(response)
        } else {
            Err(response.code.into())
        }
    }

    /// The IDN command gives brief information about the ST25R95 and its revision.
    pub fn idn(&mut self) -> Result<(&str, u16), St25r95Error<E>> {
        self.send_command(Command::Idn, &[])?;
        self.poll(None, PollFlags::CAN_READ)?;
        let response = self.read()?;
        if response.len != 15 {
            return Err(St25r95Error::InvalidResponseLength {
                expected: 15,
                actual: response.len,
            });
        }

        let resp = &self.buf[..response.len.into()];

        let idn_str = from_utf8(&resp[..13]).map_err(|_| St25r95Error::IdentificationError)?;
        let rom_crc = ((resp[13] as u16) << 8) | resp[14] as u16; // TODO: check endianness
        Ok((idn_str, rom_crc))
    }

    /// This command selects the RF communication protocol and prepares the ST25R95 for
    /// communication with a reader or contactless tag.
    pub fn select_protocol(&mut self, selection: ProtocolSelection) -> Result<(), St25r95Error<E>> {
        let mut data = [0u8; 9];
        data[0] = selection.protocol as u8;
        if selection.param_len > 0 {
            data[1..1 + selection.param_len]
                .copy_from_slice(&selection.parameters[..selection.param_len]);
        }

        self.send_command(Command::ProtocolSelect, &data[..1 + selection.param_len])?;

        // TODO? add polling

        let response = self.read()?;
        if response.len != 0 {
            Err(St25r95Error::InvalidResponseLength {
                expected: 0,
                actual: response.len,
            })
        } else {
            self.protocol = Some(selection.protocol);
            self.modulation = selection.modulation();
            Ok(())
        }
    }

    /// This command can be used to detect the presence/absence of an HF field by
    /// monitoring the field detector (FieldDet) flag. It can be used as well to wait for
    /// HF field appearance or disappearance until a defined timeout expires. The answer
    /// to the PollField command is the value of the FieldDet flag.
    pub fn poll_field(&mut self, wff: Option<WaitForField>) -> Result<bool, St25r95Error<E>> {
        match wff {
            None => self.send_command(Command::PollField, &[])?,
            Some(WaitForField {
                apparance,
                presc,
                timer,
            }) => self.send_command(Command::PollField, &[apparance as u8, presc, timer])?,
        }

        self.poll(None, PollFlags::CAN_READ)?;

        let response = self.read()?;
        match response.len {
            0 => Ok(false),
            1 => Ok(self.buf[0] & 0x01 == 1),
            other => Err(St25r95Error::InvalidResponseLength {
                expected: 1,
                actual: other,
            }),
        }
    }

    /// Read data from the remote reader through the ST25R95 in Listen mode
    pub fn read_buf(&mut self) -> Result<(u8, &[u8]), St25r95Error<E>> {
        let response = self.read()?;
        Ok((response.code, &self.buf[..response.len as usize]))
    }

    /// This command sends data to a contactless tag and receives its reply.
    pub fn send_receive(&mut self, data: &[u8]) -> Result<(u8, &[u8]), St25r95Error<E>> {
        if self.protocol.is_none() {
            return Err(St25r95Error::ProtocolNotSelected);
        }
        self.send_command(Command::SendRecv, data)?;

        // TODO? add polling

        self.read_buf()
    }

    /// In card emulation mode, this command waits for a command from an external reader.
    pub fn listen(&mut self) -> Result<(), St25r95Error<E>> {
        if self.protocol.is_none() {
            return Err(St25r95Error::ProtocolNotSelected);
        }
        self.send_command(Command::Listen, &[])?;

        // TODO? add polling

        let response = self.read()?;
        if response.len != 0 {
            Err(St25r95Error::InvalidResponseLength {
                expected: 0,
                actual: response.len,
            })
        } else {
            Ok(())
        }
    }

    /// This command immediately sends data to the reader using the Load Modulation method
    /// without waiting for a reply.
    pub fn send(&mut self, data: &[u8]) -> Result<(), St25r95Error<E>> {
        if self.protocol.is_none() {
            return Err(St25r95Error::ProtocolNotSelected);
        }
        self.send_command(Command::Send, data)?;

        // TODO? add polling

        let response = self.read()?;
        if response.len != 0 {
            Err(St25r95Error::InvalidResponseLength {
                expected: 0,
                actual: response.len,
            })
        } else {
            Ok(())
        }
    }

    fn _idle(
        &mut self,
        mut params: IdleParams,
        check_params: bool,
    ) -> Result<WakeUpSource, St25r95Error<E>> {
        if check_params && params.wus.tag_detection {
            match self.dac_ref {
                None => return Err(St25r95Error::CalibrationNeeded),
                Some(dac_ref) => {
                    params.dac_data.high =
                        dac_ref
                            .checked_add(self.dac_guard)
                            .ok_or(St25r95Error::TagDetector {
                                dac_ref,
                                dac_guard: self.dac_guard,
                            })?;
                    params.dac_data.low =
                        dac_ref
                            .checked_sub(self.dac_guard)
                            .ok_or(St25r95Error::TagDetector {
                                dac_ref,
                                dac_guard: self.dac_guard,
                            })?;
                }
            }
        }
        self.send_command(Command::Idle, &params.data())?;

        // TODO? add polling

        let response = self.read()?;
        if response.len != 1 {
            Err(St25r95Error::InvalidResponseLength {
                expected: 1,
                actual: response.len,
            })
        } else {
            self.buf[0]
                .try_into()
                .map_err(|_| St25r95Error::InvalidWakeUpSource(self.buf[0]))
        }
    }

    /// Calibrate the tag detector as wake-up source by an iterrative process.
    /// Store the DAC Ref value for further dac_data calculation using dac_guard.
    pub fn calibrate_tag_detector(&mut self) -> Result<(), St25r95Error<E>> {
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
            return Err(St25r95Error::CalibTagDetectionFailed);
        }
        params.dac_data.high = 0xFC; // max value
        let mut wus = self._idle(params, false)?;
        if !wus.timeout {
            return Err(St25r95Error::CalibTimeoutFailed);
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

    /// This command switches the ST25R95 into low power consumption mode and defines the
    /// way to return to Ready state.
    ///
    /// Caution:
    /// In low power consumption mode the device does not support SPI poll mechanism.
    /// Application has to rely on IRQ_OUT before reading the answer to the Idle command.
    pub fn idle(&mut self, params: IdleParams) -> Result<WakeUpSource, St25r95Error<E>> {
        self._idle(params, true)
    }

    fn _write_register(
        &mut self,
        reg: impl Register,
        inc_addr: bool,
        set_index_only: bool,
    ) -> Result<(), St25r95Error<E>> {
        if set_index_only {
            let mut data = [0u8; 3];
            data[0] = reg.control();
            data[1] = 0;
            data[2] = reg.data()[0];
            self.send_command(Command::WrReg, &data)?;
        } else {
            let mut data = [0u8; 4];
            data[0] = reg.control();
            data[1] = inc_addr as u8;
            data[2..].copy_from_slice(&reg.data());
            self.send_command(Command::WrReg, &data)?;
        }

        // TODO? add polling

        let response = self.read()?;
        if response.len != 0 {
            Err(St25r95Error::InvalidResponseLength {
                expected: 0,
                actual: response.len,
            })
        } else {
            Ok(())
        }
    }

    /// This command is used to read the ACC_A, ARC_B, or Wakeup register.
    pub fn read_register(&mut self, reg: ReadableRegister) -> Result<u8, St25r95Error<E>> {
        if self.protocol.is_none() {
            return Err(St25r95Error::ProtocolNotSelected);
        }
        // Set register index first
        match reg {
            ReadableRegister::AccA => {
                self._write_register(AccA::default(self.protocol.unwrap())?, true, true)?
            }
            ReadableRegister::ArcB => self._write_register(
                ArcB::default(self.protocol.unwrap(), &self.modulation)?,
                true,
                true,
            )?,
            _ => {}
        }
        let mut data = [0u8; 3];
        data[0] = match reg {
            ReadableRegister::AccA | ReadableRegister::ArcB => 0x69,
            ReadableRegister::WakeupEvent => 0x62,
        };
        data[1] = 0x01;
        data[2] = 0x00;
        self.send_command(Command::RdReg, &data)?;

        // TODO? add polling

        let response = self.read()?;
        if response.len != 1 {
            Err(St25r95Error::InvalidResponseLength {
                expected: 1,
                actual: response.len,
            })
        } else {
            Ok(self.buf[0])
        }
    }

    /// The WriteRegister command is used to:
    /// • set the Analog Register Configuration register (ArcB) value
    /// • set the Analog Register Configuration register (AccA) value
    /// • set the TimerWindow value used to improve ST25R95 demodulation when
    ///   communicating with ISO/IEC 14443 Type A tags
    /// • set the AutoDetect Filter used to help synchronization of ST25R95 with FeliCa™
    ///   tags
    // • configure the HF2RF bit
    // Note: When the HF2RF bit is ‘0’, Reader mode is possible (default mode). When set
    // to ‘1’, VPS_TX power consumption is reduced (Ready mode).
    pub fn write_register(
        &mut self,
        reg: impl Register,
        inc_addr: bool,
    ) -> Result<(), St25r95Error<E>> {
        self._write_register(reg, inc_addr, false)
    }

    /// The Echo command verifies the possibility of communication between a Host and the
    /// ST25R95. The ST25R95 will exit the listen mode upon reception of an echo command.
    /// This can be used to stop listen mode.
    pub fn echo(&mut self) -> Result<(), St25r95Error<E>> {
        self.send_command(Command::Echo, &[])?;

        self.send_control(Control::Read)?;
        let response_buf = &mut self.buf[..if self.listen_mode { 3 } else { 1 }];
        self.cb.read(response_buf).map_err(|e| SpiError(e))?;
        if self.buf[0] != Command::Echo as u8 {
            return Err(St25r95Error::EchoFailed);
        }
        if self.listen_mode {
            let response = ReadResponse::new(&self.buf[1..3].try_into().unwrap());
            if response.code != 0x85 {
                return Err(St25r95Error::EchoFailed);
            }
            if response.len != 0 {
                return Err(St25r95Error::InvalidResponseLength {
                    expected: 0,
                    actual: response.len,
                });
            }
            self.listen_mode = false; // Listening mode was cancelled by the application
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ReadResponse {
    pub code: u8,
    pub len: u16,
}

impl ReadResponse {
    pub fn new(header: &[u8; 2]) -> Self {
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
