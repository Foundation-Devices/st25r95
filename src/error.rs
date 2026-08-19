// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ST25R95 Error Handling Module
//!
//! This module provides comprehensive error handling for the ST25R95 NFC driver.
//! Errors are categorized into driver-level errors (from the Rust code) and
//! hardware-level errors (reported by the ST25R95 chip itself).
//!
//! ## Error Categories
//!
//! ### Driver Errors
//! These errors originate from the Rust driver code and indicate issues with
//! parameter validation, communication protocols, or resource management:
//!
//! - **SPI/Communication**: Problems with the underlying SPI interface
//! - **Parameter Validation**: Invalid configuration values or ranges
//! - **Resource Management**: Buffer overflows, memory issues
//! - **Calibration**: Tag detector calibration failures
//!
//! ### Hardware Errors (St25r95Error)
//! These errors are reported directly by the ST25R95 chip and indicate
//! protocol-specific issues, communication problems, or hardware faults:
//!
//! - **Protocol Errors**: Framing, CRC, timing issues
//! - **Communication Errors**: Timeout, no response, buffer overflow
//! - **Hardware Faults**: Internal errors, invalid states
//!
//! ## Error Handling Strategies
//!
//! ### Basic Error Recovery
//! ```rust
//! # use st25r95::{Error, ReadResponse, Result, St25r95Error};
//! fn handle(result: Result<ReadResponse>) -> Result<()> {
//!     match result {
//!         Ok(response) => process_response(response),
//!         Err(Error::ResponseInProgress) => {
//!             // The chip is still running the command: wait longer for that
//!             // same operation instead of sending a new one
//!             wait_longer()
//!         }
//!         Err(Error::Hw(St25r95Error::FrameTimeoutOrNoTag)) => {
//!             // No tag present, wait for a tag
//!             wait_for_tag()
//!         }
//!         Err(Error::Spi) => {
//!             // Reset the SPI interface
//!             reset_spi()
//!         }
//!         // Report anything else to the caller
//!         Err(error) => Err(error),
//!     }
//! }
//! # fn process_response(_: ReadResponse) -> Result<()> { Ok(()) }
//! # fn wait_longer() -> Result<()> { Ok(()) }
//! # fn wait_for_tag() -> Result<()> { Ok(()) }
//! # fn reset_spi() -> Result<()> { Ok(()) }
//! ```
//!
//! ### Hardware-Specific Recovery
//! ```rust
//! # use st25r95::{Error, ReadResponse, Result, St25r95Error};
//! fn handle(result: Result<ReadResponse>) -> Result<()> {
//!     match result {
//!         Err(Error::Hw(St25r95Error::CrcError)) => {
//!             // Retry once for transient CRC errors
//!             retry().map(|_| ())
//!         }
//!         Err(Error::Hw(St25r95Error::FrameTimeoutOrNoTag)) => {
//!             // Check that the field is on and a tag is present first
//!             if field_present()? {
//!                 retry().map(|_| ())
//!             } else {
//!                 Ok(())
//!             }
//!         }
//!         // ... handle other hardware errors
//!         other => other.map(|_| ()),
//!     }
//! }
//! # fn retry() -> Result<ReadResponse> {
//! #     ReadResponse::try_from([0x00, 0x00].as_slice())
//! # }
//! # fn field_present() -> Result<bool> { Ok(true) }
//! ```
//!
//! ### Calibration Error Handling
//! ```rust
//! # use st25r95::{Error, Result};
//! fn handle(result: Result<u8>) -> Result<()> {
//!     match result {
//!         Ok(dac_ref) => {
//!             // Calibration successful, tag detection can be used
//!             setup_idle_mode_with_tag_detection(dac_ref)
//!         }
//!         Err(Error::CalibTagDetectionFailed) => {
//!             // Tag detection not possible, use other wake-up sources
//!             setup_idle_mode_without_tag_detection()
//!         }
//!         // Other calibration error
//!         Err(error) => Err(error),
//!     }
//! }
//! # fn setup_idle_mode_with_tag_detection(_: u8) -> Result<()> { Ok(()) }
//! # fn setup_idle_mode_without_tag_detection() -> Result<()> { Ok(()) }
//! ```

use derive_more::From;

/// Comprehensive error type for ST25R95 operations
///
/// This enum encompasses all possible error conditions that can occur
/// during ST25R95 operation, from driver-level issues to hardware-reported
/// errors. Each error variant provides specific information about the
/// failure condition to aid in debugging and error recovery.
///
/// ## Error Recovery Guidelines
///
/// **Retry-worthy errors**: FrameTimeoutOrNoTag, CrcError
/// **In-progress operations**: ResponseInProgress (drain or discard it first)
/// **Configuration errors**: Invalid parameters, CalibrationNeeded
/// **Hardware errors**: Spi, CommunicationError (may require reset)
/// **User errors**: Invalid command sequences, parameter ranges
#[derive(Copy, Clone, Debug, From, PartialEq)]
pub enum Error {
    Spi,
    #[from]
    UTF8(core::str::Utf8Error),
    Vec,
    PollTimeout,

    /// The host stopped waiting before the ST25R95 answered
    ///
    /// The configured host deadline elapsed while IRQ_OUT was still high. The
    /// operation is not cancelled: the chip is still running it and will
    /// eventually answer. Do not send a new command, which would race that
    /// answer; wait longer with `poll_pending_response`, or give the answer up
    /// with `discard_pending_response`.
    ResponseInProgress,

    IdentificationError,
    InternalBufferOverflow,

    #[from]
    Hw(St25r95Error),

    InvalidDataLen(usize),
    InvalidModulationIndex(u8),
    InvalidReceiverGain(u8),
    InvalidDemodulatorSensitivity(u8),
    InvalidLoadModulationIndex {
        load_modulation_index: u8,
        min: u8,
        max: u8,
    },
    InvalidRFU(u8),
    InvalidU8Parameter {
        min: u8,
        max: u8,
        actual: u8,
    },
    InvalidResponseLength {
        expected: usize,
        actual: usize,
    },
    InvalidWakeUpSource(u8),
    CalibrationNeeded,
    TagDetector {
        dac_ref: u8,
        dac_guard: u8,
    },
    // Tag Detector Calibration
    CalibTagDetectionFailed, // Expected Tag Detection failed
    CalibTimeoutFailed,      // Expected Timeout failed
    CalibDacOutOfRange {
        dac_high: u8,
        delta: i16,
    },

    InvalidAntiColState(u8),
    InvalidCascadeLevelFilterCount(usize),

    EchoFailed,
}

impl From<heapless::CapacityError> for Error {
    fn from(_: heapless::CapacityError) -> Self {
        Self::Vec
    }
}

/// ST25R95 hardware-reported errors
///
/// These error codes are returned directly by the ST25R95 chip and indicate
/// protocol-specific communication problems, hardware faults, or timing issues.
/// Each error corresponds to specific conditions defined in the ST25R95
/// datasheet and provides insight into the nature of the failure.
///
/// ## Error Categories
///
/// ### Protocol-Specific Errors
/// - **ISO14443B**: EmdSOFerror23, EmdSOFerror10, EmdEgt, TrlTooBig, TrlTooSmall
/// - **FeliCa**: CrcError, InvalidLength
/// - **General**: InvalidSof, FramingError, RxBufferOverflow
///
/// ### Communication Errors
/// - **Timing**: FrameTimeoutOrNoTag, EgtTimeout
/// - **Hardware**: CommunicationError, Internal
/// - **Buffer**: RxBufferOverflow
///
/// ### Operational Errors
/// - **Configuration**: InvalidCommandLength, InvalidProtocol
/// - **User Action**: UserStop, NoField
///
/// ## Recovery Strategies
///
/// **Transient errors** (may retry):
/// - FrameTimeoutOrNoTag, CrcError, CommunicationError
///
/// **Configuration errors** (need parameter adjustment):
/// - InvalidCommandLength, InvalidProtocol, InvalidLength
///
/// **Hardware errors** (may need reset):
/// - Internal, CommunicationError, RxBufferOverflow
///
/// **Expected conditions** (handle gracefully):
/// - UserStop, NoField, FrameTimeoutOrNoTag
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum St25r95Error {
    /// SOF error in high part (duration 2 to 3 etu) in ISO/IEC 14443B
    ///
    /// Protocol: ISO14443B
    ///
    /// Cause: Start of Frame timing violation in the high part of the signal.
    /// The timing is outside the expected 2-3 Elementary Time Units (etu) range.
    ///
    /// Recovery: Retry the operation, check for RF interference.
    EmdSOFerror23,

    /// SOF error in low part (duration 10 to 11 etu) in ISO/IEC 14443B
    ///
    /// Protocol: ISO14443B
    ///
    /// Cause: Start of Frame timing violation in the low part of the signal.
    /// The timing is outside the expected 10-11 etu range.
    ///
    /// Recovery: Retry the operation, verify tag compatibility.
    EmdSOFerror10,

    /// Error Extended Guard Time error in ISO/IEC 14443B
    ///
    /// Protocol: ISO14443B
    ///
    /// Cause: Extended Guard Time (EGT) timing violation occurred during
    /// ISO14443B communication.
    ///
    /// Recovery: Adjust timing parameters, retry operation.
    EmdEgt,

    /// TR1 too long, reception stopped in ISO/IEC 14443B
    ///
    /// Protocol: ISO14443B
    ///
    /// Cause: The card's TR1 response time was longer than expected,
    /// causing the receiver to stop waiting.
    ///
    /// Recovery: Increase timeout values, check tag specifications.
    TrlTooBig,

    /// TR1 too small in ISO/IEC 14443B
    ///
    /// Protocol: ISO14443B
    ///
    /// Cause: The card's TR1 response time was shorter than expected.
    ///
    /// Recovery: Retry operation, this is usually transient.
    TrlTooSmall,

    /// Wrong frame format detected
    ///
    /// Cause: The received frame format doesn't match the expected protocol
    /// format or internal frame decoding failed.
    ///
    /// Recovery: Check protocol configuration, retry operation.
    Internal,

    /// Frame correctly received (not actually an error)
    ///
    /// Note: This indicates successful frame reception. It's included in the
    /// error enumeration for completeness but typically indicates success.
    ///
    /// Recovery: Continue normal operation.
    FrameRecvOK,

    /// Invalid command length
    ///
    /// Cause: The command length sent to the ST25R95 was invalid or doesn't
    /// match the expected format for the specific command.
    ///
    /// Recovery: Check command implementation, verify data length.
    InvalidCommandLength,

    /// Invalid protocol specified
    ///
    /// Cause: An invalid protocol identifier was used in a ProtocolSelect
    /// command or the protocol is not supported.
    ///
    /// Recovery: Use a valid protocol identifier from the Protocol enum.
    InvalidProtocol,

    /// Operation stopped by user (card emulation mode only)
    ///
    /// Protocol: Card Emulation
    ///
    /// Cause: The user cancelled the listening mode, typically by sending
    /// an Echo command while in card emulation mode.
    ///
    /// Recovery: This is expected behavior, continue with next operation.
    UserStop,

    /// Hardware communication error
    ///
    /// Cause: General hardware communication failure between the host and
    /// the ST25R95 chip.
    ///
    /// Recovery: Check SPI connections, may need hardware reset.
    CommunicationError,

    /// Frame timeout or no tag detected
    ///
    /// Cause: No valid response was received within the expected time frame.
    /// This commonly occurs when no tag is present in the RF field.
    ///
    /// Recovery: Check if tag is present, verify field is on, may retry.
    FrameTimeoutOrNoTag,

    /// Invalid Start of Frame (SOF)
    ///
    /// Cause: The received frame doesn't start with the expected SOF pattern
    /// for the current protocol.
    ///
    /// Recovery: Retry operation, check for RF interference.
    InvalidSof,

    /// Receive buffer overflow
    ///
    /// Cause: More data was received than the internal buffer can handle,
    /// typically due to unexpectedly long responses or multiple responses.
    ///
    /// Recovery: Clear buffers, retry with appropriate expectations.
    RxBufferOverflow,

    /// Framing error (invalid start/stop bits)
    ///
    /// Cause: UART-style framing error where start bit = 1 or stop bit = 0,
    /// indicating corrupted data transmission.
    ///
    /// Recovery: Retry operation, check signal integrity.
    FramingError,

    /// Extended Guard Time (EGT) timeout
    ///
    /// Cause: The Extended Guard Time period expired without the expected
    /// signal transition.
    ///
    /// Recovery: Adjust timing parameters, retry operation.
    EgtTimeout,

    /// Invalid length (FeliCa only)
    ///
    /// Protocol: FeliCa
    ///
    /// Cause: Received frame length is less than 3 bytes, which is invalid
    /// for FeliCa protocol frames.
    ///
    /// Recovery: Retry operation, verify FeliCa tag compatibility.
    InvalidLength,

    /// CRC error (FeliCa only)
    ///
    /// Protocol: FeliCa
    ///
    /// Cause: Cyclic Redundancy Check failed, indicating data corruption
    /// during transmission.
    ///
    /// Recovery: Retry operation, this is typically transient.
    CrcError,

    /// Reception lost without EOF
    ///
    /// Cause: The subcarrier signal was lost before receiving the End of
    /// Frame (EOF) marker, indicating incomplete reception.
    ///
    /// Recovery: Retry operation, check tag distance and interference.
    ReceptionLostWithoutEof,

    /// No external field detected (card emulation)
    ///
    /// Protocol: Card Emulation
    ///
    /// Cause: The Listen command detected that there's no external RF field
    /// from a reader present.
    ///
    /// Recovery: This is normal when no reader is present, continue waiting.
    NoField,

    /// Residual bits in last byte
    ///
    /// Protocol: ISO14443A
    ///
    /// Cause: The last byte contains incomplete bits, which is normal for
    /// short responses like ACK/NAK in ISO14443A.
    ///
    /// Recovery: This is usually expected behavior, continue processing.
    UintByte,

    /// Unknown error code from hardware
    ///
    /// Cause: The ST25R95 returned an error code that isn't defined in the
    /// specification.
    ///
    /// Recovery: Log the error code for investigation, retry operation.
    UnknownError(u8),
}

impl From<u8> for St25r95Error {
    fn from(code: u8) -> Self {
        match code {
            0x63 => St25r95Error::EmdSOFerror23,
            0x65 => St25r95Error::EmdSOFerror10,
            0x66 => St25r95Error::EmdEgt,
            0x67 => St25r95Error::TrlTooBig,
            0x68 => St25r95Error::TrlTooSmall,
            0x71 => St25r95Error::Internal,
            0x80 => St25r95Error::FrameRecvOK, // not really an error
            0x82 => St25r95Error::InvalidCommandLength,
            0x83 => St25r95Error::InvalidProtocol,
            0x85 => St25r95Error::UserStop,
            0x86 => St25r95Error::CommunicationError,
            0x87 => St25r95Error::FrameTimeoutOrNoTag,
            0x88 => St25r95Error::InvalidSof,
            0x89 => St25r95Error::RxBufferOverflow,
            0x8A => St25r95Error::FramingError,
            0x8B => St25r95Error::EgtTimeout,
            0x8C => St25r95Error::InvalidLength,
            0x8D => St25r95Error::CrcError,
            0x8E => St25r95Error::ReceptionLostWithoutEof,
            0x8F => St25r95Error::NoField,
            0x90 => St25r95Error::UintByte, // not really an error
            other => St25r95Error::UnknownError(other),
        }
    }
}

impl From<St25r95Error> for u8 {
    fn from(value: St25r95Error) -> Self {
        match value {
            St25r95Error::EmdSOFerror23 => 0x63,
            St25r95Error::EmdSOFerror10 => 0x65,
            St25r95Error::EmdEgt => 0x66,
            St25r95Error::TrlTooBig => 0x67,
            St25r95Error::TrlTooSmall => 0x68,
            St25r95Error::Internal => 0x71,
            St25r95Error::FrameRecvOK => 0x80,
            St25r95Error::InvalidCommandLength => 0x82,
            St25r95Error::InvalidProtocol => 0x83,
            St25r95Error::UserStop => 0x85,
            St25r95Error::CommunicationError => 0x86,
            St25r95Error::FrameTimeoutOrNoTag => 0x87,
            St25r95Error::InvalidSof => 0x88,
            St25r95Error::RxBufferOverflow => 0x89,
            St25r95Error::FramingError => 0x8A,
            St25r95Error::EgtTimeout => 0x8B,
            St25r95Error::InvalidLength => 0x8C,
            St25r95Error::CrcError => 0x8D,
            St25r95Error::ReceptionLostWithoutEof => 0x8E,
            St25r95Error::NoField => 0x8F,
            St25r95Error::UintByte => 0x90,
            St25r95Error::UnknownError(e) => e,
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;

impl core::error::Error for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::result::Result<(), core::fmt::Error> {
        write!(f, "{self:?}")
    }
}
