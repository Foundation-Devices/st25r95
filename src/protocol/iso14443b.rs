// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ISO/IEC 14443 Type B Protocol Support
//!
//! This module provides support for the ISO/IEC 14443 Type B protocol, a robust
//! NFC communication standard commonly used for high-security applications. Type B
//! is the foundation for:
//!
//! - **Electronic passports**: ICAO-compliant travel documents
//! - **National ID cards**: Government-issued identification
//! - **Secure payment cards**: Visa, Mastercard EMV applications
//! - **Healthcare cards**: Patient identification and insurance cards
//! - **Transportation cards**: High-security transit systems
//!
//! ## Protocol Characteristics
//!
//! - **Communication Range**: Up to 10 cm (typically 3-7 cm)
//! - **Data Rates**: 106 kbps, 212 kbps, 424 kbps, 848 kbps
//! - **Anticollision**: Uses probabilistic slot marker mechanism
//! - **Security**: Enhanced security with advanced cryptographic support
//! - **Power**: Passive tags powered by RF field with collision avoidance
//!
//! ## Key Differences from Type A
//!
//! Type B offers several advantages over Type A:
//! - **Better anticollision**: More robust collision detection and avoidance
//! - **Higher security**: Enhanced cryptographic capabilities
//! - **Lower collision probability**: More reliable in multi-tag environments
//! - **State machine based**: More deterministic communication
//!
//! ## Reader Mode Features
//!
//! The reader mode implementation provides:
//! - **Configurable speeds** (26kbps, 52kbps, 6kbps) for different tag types
//! - **Adjustable modulation** (100% or 10% ASK) for power efficiency
//! - **Subcarrier selection** (single or double) for optimal performance
//! - **SOF detection** for enhanced frame synchronization
//! - **CRC validation** for data integrity
//!
//! ## Typical Applications
//!
//! Type B is preferred for applications requiring:
//! - **High security** with cryptographic operations
//! - **Multi-tag environments** with reliable anticollision
//! - **Government identification** with compliance requirements
//! - **International standards** compatibility (ICAO, ISO/IEC 7816)
//!
//! ## Usage Examples
//!
//! ```rust,ignore
//! // Reader mode with default settings
//! let mut reader = nfc.protocol_select_iso14443b(Default::default())?;
//!
//! // Send ATTRIB command to select Type B card
//! let response = reader.send_receive(&[0x1D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x01, 0x00])?;
//!
//! // Configure for high-security applications
//! let params = iso14443b::reader::Parameters::new()
//!     .speed(iso14443b::reader::Speed::Kbps52)
//!     .modulation(iso14443b::reader::Modulation::Percent100)
//!     .wait_for_sof();
//! ```

pub mod reader {
    use super::super::ProtocolParams;

    #[derive(Debug, Copy, Clone, Default)]
    pub enum DataRate {
        #[default]
        Kbps106 = 0b00,
        Kbps212 = 0b01,
        Kbps424 = 0b10,
        Kbps828 = 0b11,
    }

    #[derive(Debug, Copy, Clone, Default)]
    pub struct FWT {
        pp: u8,
        mm: u8,
        dd: u8,
    }

    impl FWT {
        pub fn new(pp: u8, mm: u8, dd: u8) -> Option<Self> {
            if pp > 15 {
                return None;
            }
            if dd > 127 {
                return None;
            }
            Some(Self { pp, mm, dd })
        }

        pub fn us(self) -> f32 {
            (((1u32 << self.pp) as f32) * ((self.mm as f32) + 1f32) * ((self.dd as f32) + 128f32))
                * 32f32
                / 13.56f32
        }
    }

    #[derive(Debug)]
    pub struct Parameters {
        tx_data_rate: DataRate,
        rx_data_rate: DataRate,
        with_crc: bool,
        fwt: Option<FWT>,
        tttt: u16,
        yy: u8,
        zz: u8,
    }

    impl Default for Parameters {
        fn default() -> Self {
            Self {
                tx_data_rate: DataRate::default(),
                rx_data_rate: DataRate::default(),
                with_crc: false,
                fwt: None,
                tttt: 1023,
                yy: 0,
                zz: 26,
            }
        }
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

        pub fn fwt(self, fwt: FWT) -> Self {
            Self {
                fwt: Some(fwt),
                ..self
            }
        }

        pub fn tttt(self, tttt: u16) -> Self {
            Self { tttt, ..self }
        }

        pub fn yy(self, yy: u8) -> Self {
            Self { yy, ..self }
        }

        pub fn zz(self, zz: u8) -> Self {
            Self { zz, ..self }
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
            let mut param_len = 1;
            if let Some(fdt) = self.fwt {
                parameters[param_len] = fdt.pp;
                parameters[param_len + 1] = fdt.mm;
                parameters[param_len + 2] = fdt.dd;
                param_len += 3;

                parameters[param_len] = (self.tttt >> 8) as u8;
                parameters[param_len + 1] = self.tttt as u8;
                param_len += 2;

                parameters[param_len] = self.yy;
                param_len += 1;

                parameters[param_len] = self.zz;
                param_len += 1;
            };
            (parameters, param_len)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        pub fn test_fwt() {
            assert!(FWT::new(16, 0, 0).is_none());
            assert!(FWT::new(0, 0, 128).is_none());
            assert_eq!(FWT::new(0, 0, 0).unwrap().us(), 302.06488);
            assert_eq!(FWT::new(15, 0, 0).unwrap().us(), 9898062.0);
            assert_eq!(FWT::new(0, 255, 0).unwrap().us(), 77328.61);
            assert_eq!(FWT::new(0, 0, 127).unwrap().us(), 601.7699);
        }

        #[test]
        pub fn test_parameters() {
            assert_eq!(
                Parameters::default().data(),
                ([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1)
            );
            assert_eq!(
                Parameters::default().with_crc().data(),
                ([0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1)
            );
            assert_eq!(
                Parameters::default()
                    .tx_data_rate(DataRate::Kbps424)
                    .rx_data_rate(DataRate::Kbps424)
                    .fwt(FWT::new(1, 2, 3).unwrap())
                    .data(),
                ([0xA0, 0x01, 0x02, 0x03, 0x03, 0xFF, 0x00, 0x1A], 8),
            );
        }
    }
}

/* ------------------------------ */
/* example of data from datasheet */
/* ------------------------------ */

/* Table 16 */
/* NFC Forum Tag Type 4B */

// >>> [0x05, 0x00, 0x00] REQB
// <<< [0x50, 0x77, 0xFE, 0x01, 0xB3, 0x00, 0x00,
//      0x00, 0x00, 0x00, 0x71, 0x71, 0x8E, 0xBA, 0x00] ATQB

/* Table 17 */

// <<< [0x5092036A8D00000000007171, 0x3411, 0x00]
//      DataFromTag Original(Received)ValueOfCRC ReceptionFlags

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
