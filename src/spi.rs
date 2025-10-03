// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{Command, PollFlags, ReadResponse, Result};

pub trait St25r95Spi {
    fn poll(&mut self, flags: PollFlags) -> Result<()>;
    fn reset(&mut self) -> Result<()>;
    fn send_command(&mut self, cmd: Command, data: &[u8]) -> Result<()>;
    fn read(&mut self) -> Result<ReadResponse>;
    fn read_echo(&mut self) -> Result<()>;
}

/*
pub struct SpiAdapter {
    spi: SpiDevice,
    buf: [u8; SPI_MAX_XFER_LEN],
}

impl SpiAdapter {
    pub fn new(spi: SpiDevice) -> Self {
        Self {
            spi,
            buf: [0u8; SPI_MAX_XFER_LEN],
        }
    }
}

impl St25r95Spi for SpiAdapter {
    fn poll(&mut self, flags: PollFlags) -> Result<()> {
        let mut curr_flags = [0; 2];
        self.spi
            .transfer(&mut curr_flags, &[Control::Poll as u8, Control::Poll as u8])
            .map_err(Error::Spi)?;
        match PollFlags::from_bits_truncate(curr_flags[1]).contains(flags) {
            true => Ok(()),
            false => Err(Error::PollTimeout),
        }
    }

    fn reset(&mut self) -> Result<()> {
        self.spi.write(&[Control::Reset as u8]).map_err(Error::Spi)
    }

    fn send_command(&mut self, cmd: Command, data: &[u8]) -> Result<()> {
        if data.len() > self.buf.len() - 3 {
            return Err(Error::InternalBufferOverflow);
        }

        self.buf[0] = Control::Send as u8;
        self.buf[1] = cmd as u8;
        // TODO: special case of Echo, do not sent the len
        self.buf[2] = data.len() as u8;
        self.buf[3..3 + data.len()].copy_from_slice(data);

        self.spi
            .write(&self.buf[..3 + data.len()])
            .map_err(Error::Spi)
    }

    fn read(&mut self) -> Result<ReadResponse> {
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

    fn read_echo(&mut self) -> Result<()>  {
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
}
*/
