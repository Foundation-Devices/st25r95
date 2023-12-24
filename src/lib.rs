// SPDX-FileCopyrightText: 2023 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(not(test), no_std)]

mod callbacks;
mod command;
mod control;
mod error;
mod protocol;

pub use crate::error::St25r95Error;
pub use crate::callbacks::Callbacks;
pub use crate::protocol::*;
pub use crate::command::{PollParams};
pub use crate::control::PollFlags;

use crate::command::{Command};
use crate::control::{Control};
use crate::error::St25r95Error::SpiError;
use core::fmt::Debug;
use core::str::from_utf8;

pub struct St25r95<'a, E: Debug, C: Callbacks<Error = E>> {
    cb: C,
    buf: &'a mut [u8],
}

const TIMING_T0: u8 = 1;
const TIMING_T1: u8 = 1;
const TIMING_T3: u8 = 10;

impl<'a, E: Debug, C: Callbacks<Error = E>> St25r95<'a, E, C> {
    pub fn new(cb: C, buf: &'a mut [u8]) -> Self {
        Self { cb, buf }
    }

    pub fn init(&mut self) -> Result<(), St25r95Error<E>> {
        self.reset()?;
        self.verify_idn()?;
        Ok(())
    }

    fn verify_idn(&mut self) -> Result<(), St25r95Error<E>> {
        self.send_command(Command::Idn, &[])?;
        self.poll(None, PollFlags::CAN_READ)?;
        let response = self.read(false)?;
        if response.code != 0x00 {
            return Err(St25r95Error::IdentificationError);
        }

        let buf = &self.buf[..response.len.into()];

        let idn_str = from_utf8(&buf[..12]).map_err(|_| St25r95Error::IdentificationError)?;
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
        }
    }

    fn read(&mut self, is_echo: bool) -> Result<ReadResponse, St25r95Error<E>> {
        self.irq_pulse();
        self.send_control(Control::Read)?;

        let response_header_buf = &mut self.buf[..2];
        self.cb
            .read(response_header_buf)
            .map_err(|e| SpiError(e))?;

        let mut response = ReadResponse::new(response_header_buf[0], response_header_buf[1]);

        // Handle special case for echo command + error (0x85 = listening cancelled)
        if response.code == 0x55 && is_echo {
            if response_header_buf[1] == 0x85 {
                response.code = 0x85;
            }
            response.len = 0;
        }

        if response.len != 0 {
            if response.len as usize > self.buf.len() {
                return Err(St25r95Error::InternalBufferOverflow);
            }

            self.cb
                .read(&mut self.buf[..response.len as usize])
                .expect("spi transfer");
        }
        self.cb.release();

        Ok(response)
    }

    pub fn select_protocol(&mut self, selection: ProtocolSelection) -> Result<(), St25r95Error<E>> {
        let mut data = [0u8; 9];
        data[0] = selection.protocol as u8;
        data[1..1 + selection.param_len].copy_from_slice(&selection.parameters[..selection.param_len]);

        self.send_command(Command::ProtocolSelect, &data[..1 + selection.param_len])?;

        let response = self.read(false)?;
        match response.code {
            0 => Ok(()),
            0x82 => Err(St25r95Error::InvalidCommandLength),
            0x83 => Err(St25r95Error::InvalidProtocol),
            other => Err(St25r95Error::UnknownError(other)),
        }
    }

    pub fn field_off(&mut self) -> Result<(), St25r95Error<E>> {
        self.send_command(Command::ProtocolSelect, &[0x00])
    }

    pub fn card_emulation_listen(&mut self) -> Result<(), St25r95Error<E>> {
        self.send_command(Command::Listen, &[])?;

        let response = self.read(false)?;
        match response.code {
            0x00 => Ok(()),
            0x82 => Err(St25r95Error::InvalidCommandLength),
            0x83 => Err(St25r95Error::InvalidProtocol),
            0x8F => Err(St25r95Error::NoField),
            other => Err(St25r95Error::UnknownError(other)),
        }
    }

    pub fn poll_field(&mut self, poll_params: PollParams) -> Result<bool, St25r95Error<E>> {
        match poll_params {
            PollParams::NoParams => self.send_command(Command::PollField, &[])?,
            PollParams::WaitForField { presc, timer } =>
                self.send_command(Command::PollField, &[0x01, presc, timer])?,
        }

        self.poll(None, PollFlags::CAN_READ)?;

        let response = self.read(false)?;
        match response.code {
            0x00 => {
                if response.len == 0 {
                    Ok(false)
                } else {
                    Ok(self.buf[0] == 1)
                }
            }
            0x82 => Err(St25r95Error::InvalidCommandLength),
            other => Err(St25r95Error::UnknownError(other)),
        }
    }

    pub fn read_buf(&mut self) -> Result<(u8, &[u8]), St25r95Error<E>> {
        let response = self.read(false)?;
        Ok((response.code, &self.buf[..response.len as usize]))
    }

    pub fn write_reg(&mut self, data: &[u8]) -> Result<(), St25r95Error<E>> {
        self.send_command(Command::WrReg, data)?;
        let response = self.read(false)?;
        match response.code {
            0x00 => Ok(()),
            other => Err(St25r95Error::UnknownError(other)),
        }
    }

    pub fn send_receive(&mut self, data: &[u8]) -> Result<(u8, &[u8]), St25r95Error<E>> {
        self.send_command(Command::SendRecv, data)?;
        let (code, buf) = self.read_buf()?;

        match code {
            0x86 => Err(St25r95Error::CommunicationError),
            0x87 => Err(St25r95Error::FrameTimeoutOrNoTag),
            0x88 => Err(St25r95Error::InvalidSof),
            0x89 => Err(St25r95Error::RxBufferOverflow),
            0x8A => Err(St25r95Error::FramingError),
            0x8B => Err(St25r95Error::EgtTimeout),
            0x8C => Err(St25r95Error::InvalidLength),
            0x8D => Err(St25r95Error::CrcError),
            0x8E => Err(St25r95Error::ReceptionLostWithoutEof),

            0x80 | 0x90 => Ok((code, buf)),

            other => Err(St25r95Error::UnknownError(other)),
        }
    }

    pub fn echo(&mut self) -> Result<bool, St25r95Error<E>> {
        self.send_command(Command::Echo, &[])?;

        let response = self.read(true)?;
        match response.code {
            0x55 => Ok(false),
            0x85 => Ok(true),  // Listening was cancelled
            other => Err(St25r95Error::UnknownError(other)),
        }
    }

    pub fn calibrate(&mut self) -> Result<(), St25r95Error<E>> {
        todo!()
    }

    pub fn idle(&mut self) -> Result<(), St25r95Error<E>> {
        todo!()
    }
}

#[derive(Debug)]
struct ReadResponse {
    pub code: u8,
    pub len: u16,
}

impl ReadResponse {
    pub fn new(code: u8, len: u8) -> Self {
        // See datasheet section 4.3 (Support of long frames)
        let has_longer_len = code >> 7 & 1 != 0;
        let len = if has_longer_len {
            let extra_bits = (code as u16 >> 5) & 0b11;
            (extra_bits << 8) | len as u16
        } else {
            len as u16
        };

        Self { code, len }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::ops::Range;

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
            let res = ReadResponse::new(code, len).len;
            assert!(res_range.contains(&res))
        }
    }
}
