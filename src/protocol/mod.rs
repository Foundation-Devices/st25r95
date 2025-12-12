// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! NFC Protocol Support Module
//!
//! This module provides support for various NFC protocols that the ST25R95 can handle.
//! Each protocol has specific characteristics, timing requirements, and configuration
//! options optimized for different types of NFC tags and applications.
//!
//! ## Supported Protocols
//!
//! ### Reader Mode (communicating with external tags):
//! - **ISO/IEC 15693**: Vicinity cards for long-range applications
//! - **ISO/IEC 14443-A**: Type A tags including MIFARE, NTAG series
//! - **ISO/IEC 14443-B**: Type B tags for transit and access control
//! - **FeliCa**: Sony's proprietary protocol for Japanese markets
//!
//! ### Card Emulation Mode (emulating a tag):
//! - **ISO/IEC 14443-A**: Type A card emulation with anti-collision support
//!
//! ## Protocol Selection
//!
//! Each protocol is selected via the `ProtocolSelect` command with specific
//! parameters that configure the ST25R95 for optimal performance with that
//! protocol. The driver provides type-safe parameter structures for each protocol.
//!
//! ## Usage Examples
//!
//! ```rust,ignore
//! // Select ISO14443A reader mode with default parameters
//! let mut reader = nfc.protocol_select_iso14443a(Default::default())?;
//!
//! // Select ISO15693 with specific modulation
//! let params = iso15693::reader::Parameters::new()
//!     .with_modulation(Modulation::Percent10);
//! let mut reader = nfc.protocol_select_iso15693(params)?;
//! ```

pub mod felica;
pub mod iso14443a;
pub mod iso14443b;
pub mod iso15693;

/// Enumeration of supported NFC protocols for the ST25R95
///
/// Each protocol variant corresponds to a specific configuration of the ST25R95
/// hardware and enables different communication capabilities. The numeric values
/// are used directly in the ProtocolSelect command.
///
/// See ST25R95 datasheet Table 11 for complete protocol specifications.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Protocol {
    /// Turn off the RF field and disable RF communication
    ///
    /// This protocol mode stops the RF field generation and puts the ST25R95
    /// in a low-power state. Use this to:
    /// - Save power when not communicating
    /// - Switch between different protocols
    /// - Reset the communication state
    FieldOff = 0x00,

    /// ISO/IEC 15693 "Vicinity" card protocol
    ///
    /// Long-range protocol (up to 1.5m) for:
    /// - Inventory management and asset tracking
    /// - Library books and retail items  
    /// - Animal identification
    /// - Access control at distances
    ///
    /// Features:
    /// - Supports 10% and 100% modulation
    /// - Single and multiple card anticollision
    /// - High data rates (26 kbps)
    /// - Robust error detection
    Iso15693 = 0x01,

    /// ISO/IEC 14443 Type A protocol
    ///
    /// Short-range protocol (up to 10cm) for:
    /// - MIFARE Classic/Plus/ULTRALIGHT tags
    /// - NTAG series (NTAG213, NTAG215, NTAG216)
    /// - NFC Forum Type 1-4 tags
    /// - Payment cards and access cards
    ///
    /// Features:
    /// - Fast anticollision and UID selection
    /// - Supports MIFARE authentication
    /// - High data rates (106-848 kbps)
    /// - Widely adopted standard
    Iso14443A = 0x02,

    /// ISO/IEC 14443 Type B protocol
    ///
    /// Short-range protocol (up to 10cm) for:
    /// - Calypso transit cards
    /// - Government ID cards
    /// - Some access control systems
    /// - Electronic passports
    ///
    /// Features:
    /// - Different anticollision mechanism than Type A
    /// - Asynchronous communication
    /// - Better noise immunity than Type A
    /// - Used in official document applications
    Iso14443B = 0x03,

    /// FeliCa protocol (Sony proprietary)
    ///
    /// High-speed protocol for:
    /// - Japanese transit cards (Suica, Pasmo, ICOCA)
    /// - Mobile payments (Osaifu-Keitai)
    /// - Electronic money systems
    /// - NFC applications in Japanese market
    ///
    /// Features:
    /// - Very high data rates (212/424 kbps)
    /// - Advanced security features
    /// - Fast polling and communication
    /// - Wide adoption in Japan and Asia
    FeliCa = 0x04,

    /// Card emulation mode with ISO/IEC 14443-A protocol
    ///
    /// Configures the ST25R95 to act as an ISO14443-A tag/card for:
    /// - Payment card emulation
    /// - Access token emulation  
    /// - Peer-to-peer NFC communication
    /// - Custom tag implementations
    ///
    /// Features:
    /// - Anti-collision filter support
    /// - Configurable UID and responses
    /// - Load modulation control
    /// - Selective card presence emulation
    CardEmulationIso14443A = 0x12,
}

/// Trait for protocol parameter configuration
///
/// This trait is implemented by parameter structures for each protocol
/// to provide the binary data required by the ProtocolSelect command.
/// Each protocol has different parameter requirements and configurations.
pub(crate) trait ProtocolParams {
    /// Convert protocol parameters to binary data for ST25R95
    ///
    /// Returns a tuple containing:
    /// - Array of up to 8 parameter bytes
    /// - Actual number of valid bytes in the array
    fn data(self) -> ([u8; 8], usize);
}

/// Parameter structure for FieldOff protocol
///
/// This parameter structure is used when turning off the RF field.
/// No additional configuration is needed beyond the protocol identifier.
pub(crate) struct FieldOff;

impl ProtocolParams for FieldOff {
    fn data(self) -> ([u8; 8], usize) {
        ([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1)
    }
}
