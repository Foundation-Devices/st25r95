// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! FeliCa Protocol Support
//!
//! This module provides support for the FeliCa contactless smart card protocol,
//! developed by Sony and widely used in Japan and other Asian markets. FeliCa
//! is known for its speed and reliability, powering:
//!
//! - **Transit systems**: Suica, Pasmo, ICOCA railway cards
//! - **Electronic money**: Edy, nanaco, Waon payment systems
//! - **Access control**: Office and building entry systems
//! - **Student ID cards**: University and school identification
//! - **Loyalty programs**: Retail point-of-sale systems
//! - **Mobile payments**: Osaifu-Keitai wallet integration
//!
//! ## Protocol Characteristics
//!
//! - **Communication Range**: Up to 10 cm (typically 1-5 cm)
//! - **Data Rates**: 212 kbps, 424 kbps (high-speed)
//! - **Anticollision**: Time division multiple access (TDMA)
//! - **Security**: DES/3DES/AES encryption with mutual authentication
//! - **Power**: Passive cards with optional active modes
//! - **Transaction speed**: Extremely fast (~0.15 seconds)
//!
//! ## Key Features
//!
//! FeliCa offers several unique advantages:
//! - **High-speed communication** enables quick transactions
//! - **Reliable anticollision** with minimal delay
//! - **Strong security** with robust encryption
//! - **Flexible memory structure** with multiple services
//! - **Dual-directional communication** for complex transactions
//!
//! ## Memory Organization
//!
//! FeliCa cards organize data into:
//! - **System codes**: 16-bit identifiers for applications
//! - **Services**: Grouped memory blocks with access controls
//! - **Blocks**: 16-byte memory units for data storage
//! - **IDm**: Manufacturer-assigned unique identifier
//! - **PMm**: Parameter memory for card-specific data
//!
//! ## Reader Mode Features
//!
//! The reader mode implementation provides:
//! - **Configurable data rates** (212/424 kbps) for optimal performance
//! - **Fine-tuned FWT (Frame Waiting Time)** for timing optimization
//! - **CRC validation** and error detection
//! - **Polling frame configuration** for card detection
//!
//! ## Security Model
//!
//! FeliCa implements multi-layered security:
//! - **ID authentication** verifies card identity
//! - ** mutual authentication** with challenge-response
//! - **Encryption** for sensitive data protection
//! - **Access controls** for memory permissions
//!
//! ## Typical Applications
//!
//! FeliCa excels in:
//! - **High-throughput environments** like transit gates
//! - **Quick transactions** requiring minimal delay
//! - **Secure payment systems** with encryption
//! - **Multi-application cards** with diverse services
//!
//! ## Usage Examples
//!
//! ```rust,ignore
//! // Reader mode with default settings
//! let mut reader = nfc.protocol_select_felica(Default::default())?;
//!
//! // Polling command to detect FeliCa cards
//! let polling = [0x00, 0xFF, 0xFF, 0x01, 0x00]; // System code: FFFE (wildcard)
//! let response = reader.send_receive(&polling)?;
//!
//! // Configure for high-speed transactions
//! let params = felica::reader::Parameters::new()
//!     .tx_data_rate(felica::reader::DataRate::Kbps424)
//!     .rx_data_rate(felica::reader::DataRate::Kbps424)
//!     .with_crc(false); // FeliCa handles its own CRC
//! ```

pub mod reader {
    use super::super::ProtocolParams;

    #[derive(Debug, Copy, Clone, Default)]
    pub enum DataRate {
        #[default]
        Kbps106 = 0b00,
        Kbps212 = 0b01,
        Kbps424 = 0b10,
    }

    #[derive(Debug, Copy, Clone, Default)]
    pub struct RWT {
        pp: u8,
        mm: u8,
    }

    impl RWT {
        pub fn new(pp: u8, mm: u8) -> Option<Self> {
            if pp > 15 {
                return None;
            }
            Some(Self { pp, mm })
        }

        pub fn us(self) -> f32 {
            (((1u32 << self.pp) as f32) * ((self.mm as f32) + 1f32)) * 4096f32 / 13.56f32
        }
    }

    #[derive(Debug, Default)]
    pub struct Parameters {
        tx_data_rate: DataRate,
        rx_data_rate: DataRate,
        with_crc: bool,
        rwt: Option<RWT>,
    }

    impl Parameters {
        pub fn tx_data_rate(self, tx_data_rate: DataRate) -> Self {
            Self {
                tx_data_rate,
                ..self
            }
        }

        pub fn rx_data_rate(self, rx_data_rate: DataRate) -> Self {
            Self {
                rx_data_rate,
                ..self
            }
        }

        pub fn with_crc(self) -> Self {
            Self {
                with_crc: true,
                ..self
            }
        }

        pub fn rwt(self, rwt: RWT) -> Self {
            Self {
                rwt: Some(rwt),
                ..self
            }
        }
    }

    impl ProtocolParams for Parameters {
        fn data(self) -> ([u8; 8], usize) {
            let mut parameters = [0; 8];
            let mut param_byte = 0;
            let tx_data_rate_bits = self.tx_data_rate as u8;
            param_byte |= tx_data_rate_bits << 6;
            let rx_data_rate_bits = self.rx_data_rate as u8;
            param_byte |= rx_data_rate_bits << 4;
            let crc_bit = self.with_crc as u8;
            param_byte |= crc_bit;

            parameters[0] = param_byte;
            parameters[1] = 0x10;
            let param_len = if let Some(rwt) = self.rwt {
                parameters[2] = rwt.pp;
                parameters[3] = rwt.mm;
                4
            } else {
                2
            };
            (parameters, param_len)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        pub fn test_rwt() {
            assert!(RWT::new(16, 0).is_none());
            assert_eq!(RWT::new(0, 0).unwrap().us(), 302.06488);
            assert_eq!(RWT::new(15, 0).unwrap().us(), 9898062.0);
            assert_eq!(RWT::new(0, 255).unwrap().us(), 77328.61);
        }

        #[test]
        pub fn test_parameters() {
            assert_eq!(
                Parameters::default().data(),
                ([0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 2)
            );
            assert_eq!(
                Parameters::default().with_crc().data(),
                ([0x01, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 2)
            );
            assert_eq!(
                Parameters::default()
                    .tx_data_rate(DataRate::Kbps212)
                    .rx_data_rate(DataRate::Kbps212)
                    .with_crc()
                    .rwt(RWT::new(1, 2).unwrap())
                    .data(),
                ([0x51, 0x10, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00], 4)
            );
        }
    }
}

/* ------------------------------ */
/* example of data from datasheet */
/* ------------------------------ */

/* NFC Forum Tag Type 3 */

/* Table 16 */

// >>> [0x00, 0xFF, 0xFF, 0x00, 0x00] SENSF_REQ
// <<< [0x01, 0x01, 0x01, 0x02, 0x14, 0x8E,
//      0x0D, 0xB4, 0x13, 0x10, 0x0B, 0x4B,
//      0x42, 0x84, 0x85, 0xD0, 0xFF, 0x00] SENSF_RES

/* Table 17 */

// <<< [0x01010105017B06941004014B024F4993FF, 0x00] DataReceivedFromTag(NotIncludingCRC)
// ReceptionFlags

pub struct ReceptionFlags {
    crc_error: bool,
}

impl TryFrom<u8> for ReceptionFlags {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let crc_error = value & 0b0000_0010 != 0;
        if value & 0b1111_1101 == 0 {
            Ok(ReceptionFlags { crc_error })
        } else {
            Err(())
        }
    }
}

impl From<ReceptionFlags> for u8 {
    fn from(lbf: ReceptionFlags) -> Self {
        (lbf.crc_error as u8) << 1
    }
}
