// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use {
    crate::ReadResponse,
    derive_more::From,
    embedded_hal::{
        digital::{InputPin, OutputPin},
        spi::SpiDevice,
    },
};

#[derive(Copy, Clone, Debug, From, PartialEq)]
pub enum Error<SPI, I, O> {
    // #[from]
    Spi(SPI),
    // #[from]
    IrqOut(I),
    // #[from]
    IrqIn(O),
    #[from]
    UTF8(core::str::Utf8Error),
    PollTimeout,
    IdentificationError,
    InternalBufferOverflow,

    Hw(St25r95Error),

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
        expected: u16,
        actual: ReadResponse,
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

    InvalidAntiColState(u8),
    InvalidCascadeLevelFilterCount(usize),

    EchoFailed,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum St25r95Error {
    EmdSOFerror23,        // SOF error in high part (duration 2 to 3 etu) in ISO/IEC 14443B
    EmdSOFerror10,        // SOF error in low part (duration 10 to 11 etu) in ISO/IEC 14443B
    EmdEgt,               // Error Extended Guard Time error in ISO/IEC 14443B
    TrlTooBig,            // Too long TR1 send by the card, reception stopped in ISO/IEC 14443BT
    TrlTooSmall,          // Too small TR1 send by the card in ISO/IEC 14443B
    Internal,             // Wong frame format decodes
    FrameRecvOK,          // Frame correctly received (additionally see CRC/Parity information)
    InvalidCommandLength, // Invalid command length
    InvalidProtocol,      // Invalid protocol
    UserStop,             // Stopped by user (used only in Card mode)
    CommunicationError,   // Hardware communication error
    FrameTimeoutOrNoTag,  // Frame wait time out (no valid reception)
    InvalidSof,           // Invalid SOF
    RxBufferOverflow,     // Too many bytes received and data still arriving
    FramingError,         // if start bit = 1 or stop bit = 0
    EgtTimeout,           // EGT time out
    InvalidLength,        // Valid for FeliCa™, if Length <3
    CrcError,             // CRC error, Valid only for FeliCa™
    ReceptionLostWithoutEof, // When reception is lost without EOF received (or subcarrier was lost)
    NoField,              // When Listen command detects the absence of external field
    UintByte,             /* Residual bits in last byte. Useful for ACK/NAK reception of
                           * ISO/IEC 14443 Type A. */
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
            // 0x80 => St25r95Error::FrameRecvOK, // not really an error
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
            // 0x90 => St25r95Error::UintByte, // not really an error
            other => St25r95Error::UnknownError(other),
        }
    }
}

pub type Result<T, SPI: SpiDevice, I: InputPin, O: OutputPin> =
    core::result::Result<T, Error<SPI::Error, I::Error, O::Error>>;
