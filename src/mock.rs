// SPDX-FileCopyrightText: 2026 Foundation Devices, Inc. <hello@foundation.xyz>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Mock [`St25r95Spi`] and [`St25r95Gpio`] implementations
//!
//! These exist so the examples in this crate's documentation compile and run
//! without hardware, and so consumers can unit-test code built on the driver.
//! They answer every command with a well-formed, successful response and never
//! fail; they do not emulate the ST25R95 and must not be used to validate
//! protocol behaviour.
//!
//! ```rust
//! use st25r95::{
//!     mock::{MockGpio, MockSpi},
//!     St25r95,
//! };
//!
//! let mut nfc = St25r95::new(MockSpi::default(), MockGpio)?;
//! let (name, _rom_crc) = nfc.idn()?;
//! assert!(name.starts_with("NFC"));
//! # Ok::<(), st25r95::Error>(())
//! ```

use crate::{
    Command,
    Error,
    LFOFreq,
    PollFlags,
    ReadResponse,
    Result,
    St25r95Gpio,
    St25r95Spi,
    WakeUpSource,
    ECHO_BYTE,
    ECHO_RESPONSE_MAX_LEN,
    MAX_COMMAND_DATA_LEN,
};

/// Value the mock answers to a register read
///
/// A valid `ARC_B` byte: 17% modulation index, 27 dB receiver gain.
const ARC_B: u8 = 0x23;

/// Identification the mock answers to the `IDN` command
///
/// Thirteen bytes of chip name followed by the two-byte ROM CRC, as the
/// ST25R95 returns them.
const IDN: &[u8] = b"NFC FS2JAST4\0\x00\x01";

/// SPI implementation that answers every command successfully
///
/// The response is chosen from the last command sent, so a driver sequence
/// runs to completion: `IDN` returns a plausible identification, `RdReg`
/// returns a register value, `SendRecv` returns a short payload, and the
/// commands that are only acknowledged return an empty successful frame.
///
/// Payloads that exceed [`MAX_COMMAND_DATA_LEN`] are rejected, like a
/// conforming implementation would.
#[derive(Debug, Default)]
pub struct MockSpi {
    last_command: Option<Command>,
}

impl St25r95Spi for MockSpi {
    fn poll(&mut self, _flags: PollFlags) -> Result<()> {
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.last_command = None;
        Ok(())
    }

    fn send_command(&mut self, cmd: Command, data: &[u8], _sod: bool) -> Result<()> {
        if data.len() > MAX_COMMAND_DATA_LEN {
            return Err(Error::InvalidDataLen(data.len()));
        }
        self.last_command = Some(cmd);
        Ok(())
    }

    fn read_data(&mut self) -> Result<ReadResponse> {
        let mut data = heapless::Vec::new();
        match self.last_command.take() {
            Some(Command::Idn) => data.extend_from_slice(IDN)?,
            Some(Command::RdReg) => data.push(ARC_B).map_err(|_| Error::Vec)?,
            Some(Command::PollField) => data.push(0x01).map_err(|_| Error::Vec)?,
            Some(Command::Idle) => {
                let wake_up = WakeUpSource {
                    lfo_freq: LFOFreq::KHz32,
                    timeout: true,
                    ..Default::default()
                };
                data.push(wake_up.into()).map_err(|_| Error::Vec)?;
            }
            Some(Command::SendRecv) => data.extend_from_slice(&[0x44, 0x00])?,
            _ => {}
        }
        Ok(ReadResponse { code: 0, data })
    }

    fn read_echo(&mut self, buf: &mut [u8; ECHO_RESPONSE_MAX_LEN]) -> Result<usize> {
        buf[0] = ECHO_BYTE;
        Ok(1)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// GPIO implementation whose IRQ_OUT is always ready
///
/// Every wait succeeds immediately, so a mocked command never reports
/// [`Error::ResponseInProgress`].
#[derive(Debug, Default)]
pub struct MockGpio;

impl St25r95Gpio for MockGpio {
    fn irq_in_pulse_low(&mut self) {}

    fn wait_irq_out_falling_edge(&mut self, _timeout: u32) -> core::result::Result<(), ()> {
        Ok(())
    }
}
