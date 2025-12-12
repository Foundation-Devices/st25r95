// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{Command, PollFlags, ReadResponse, Result};

/// Trait for SPI communication with the ST25R95 NFC transceiver
///
/// This trait abstracts the low-level SPI interface required to communicate with
/// the ST25R95 chip. Implementations must handle the specific timing requirements
/// and protocol details outlined in the ST25R95 datasheet.
///
/// ## Implementation Notes
///
/// - The `poll()` method must implement the specific polling protocol
/// - `send_command()` should handle the complete command transmission sequence
/// - `read_data()` must parse response format according to ST25R95 specification
/// - Error handling should convert hardware errors to appropriate `Error` variants
///
/// ## Example Implementation
///
/// ```rust,ignore
/// struct MySpi {
///     spi: embedded_hal::spi::Spi<SPI, ...>,
///     cs: embedded_hal::digital::v2::OutputPin<CS>,
/// }
///
/// impl St25r95Spi for MySpi {
///     fn poll(&mut self, flags: PollFlags) -> Result<()> {
///         // Assert CS
///         // Send polling command
///         // Deassert CS
///         // Return success if flags are present, error on timeout
///     }
///
///     fn send_command(&mut self, cmd: Command, data: &[u8], sod: bool) -> Result<()> {
///         // Assert CS
///         // Send control byte
///         // Send command byte
///         // Send data length
///         // Send data bytes
///         // Deassert CS
///     }
///
///     // ... implement other methods
/// }
/// ```
pub trait St25r95Spi {
    /// Poll the ST25R95 status register with specified flags
    ///
    /// This method sends a polling command and read the
    /// specified flags status of the ST25R95 hardware.
    ///
    /// ## Parameters
    /// - `flags`: Bit flags indicating which conditions to wait for
    ///
    /// ## Returns
    /// - `Ok(())`: All specified flags have been cleared
    /// - `Err(Error::PollTimeout)`: Timeout occurred while waiting
    /// - `Err(Error::SpiError)`: SPI communication error occurred
    fn poll(&mut self, flags: PollFlags) -> Result<()>;

    /// Reset the ST25R95 chip via SPI interface
    ///
    /// This method performs a soft reset of the ST25R95 by sending the appropriate
    /// SPI commands. The chip will return to the Power-up state after reset.
    ///
    /// ## Returns
    /// - `Ok(())`: Reset completed successfully
    /// - `Err(Error::SpiError)`: SPI communication error
    fn reset(&mut self) -> Result<()>;

    /// Send a command to the ST25R95 with optional data
    ///
    /// This method transmits a complete command packet to the ST25R95 including
    /// the command byte, data length (if applicable), and payload data.
    ///
    /// ## Parameters
    /// - `cmd`: The command to send (see `Command` enum)
    /// - `data`: Optional data payload (can be empty)
    /// - `sod`: Start of Data flag - set to `true` for first packet in a sequence
    ///
    /// ## Returns
    /// - `Ok(())`: Command transmitted successfully
    /// - `Err(Error::SpiError)`: SPI communication error
    /// - `Err(Error::InvalidDataLen)`: Data length exceeds maximum
    ///
    /// ## Protocol Details
    /// - Control byte is sent first
    /// - Commands byte and length byte follow
    /// - Data payload is transmitted last
    /// - CS should be managed appropriately during transmission
    fn send_command(&mut self, cmd: Command, data: &[u8], sod: bool) -> Result<()>;

    /// Read response data from the ST25R95
    ///
    /// This method reads and parses a complete response packet from the ST25R95.
    /// The response format includes status/error codes and optional data payload.
    ///
    /// ## Returns
    /// - `Ok(ReadResponse)`: Successfully parsed response
    /// - `Err(Error::SpiError)`: SPI communication error
    /// - `Err(Error::InvalidResponseFormat)`: Malformed response packet
    ///
    /// ## Response Format
    /// The response follows the ST25R95 protocol:
    /// - Control byte
    /// - Status byte (error flags and status indicators)
    /// - Data length
    /// - Data payload (if length > 0)
    fn read_data(&mut self) -> Result<ReadResponse>;

    /// Flush any pending SPI data or clear the SPI buffer
    ///
    /// This method is used to clean up the SPI interface state, particularly
    /// after communication errors or timeouts. It should ensure that the SPI
    /// interface is in a known good state for subsequent operations.
    ///
    /// ## Returns
    /// - `Ok(())`: Flush completed successfully
    /// - `Err(Error::SpiError)`: Error occurred during flush
    ///
    /// ## When to Use
    /// - After communication timeouts
    /// - Before retrying failed operations
    /// - When transitioning between different operation modes
    /// - As part of error recovery procedures
    fn flush(&mut self) -> Result<()>;
}
