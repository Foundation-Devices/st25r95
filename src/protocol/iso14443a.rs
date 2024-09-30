// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

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
    pub struct FDT {
        pp: u8,
        mm: u8,
        dd: u8,
    }

    impl FDT {
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

    #[derive(Debug, Default)]
    pub struct Parameters {
        tx_data_rate: DataRate,
        rx_data_rate: DataRate,
        fdt: Option<FDT>,
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

        pub fn fdt(self, fdt: FDT) -> Self {
            Self {
                fdt: Some(fdt),
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

            parameters[0] = param_byte;
            let param_len = if let Some(fdt) = self.fdt {
                parameters[1] = fdt.pp;
                parameters[2] = fdt.mm;
                parameters[3] = fdt.dd;
                4
            } else {
                1
            };
            (parameters, param_len)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        pub fn test_fdt() {
            assert!(FDT::new(16, 0, 0).is_none());
            assert!(FDT::new(0, 0, 128).is_none());
            assert_eq!(FDT::new(0, 0, 0).unwrap().us(), 302.06488);
            assert_eq!(FDT::new(15, 0, 0).unwrap().us(), 9898062.0);
            assert_eq!(FDT::new(0, 255, 0).unwrap().us(), 77328.61);
            assert_eq!(FDT::new(0, 0, 127).unwrap().us(), 601.7699);
        }

        #[test]
        pub fn test_parameters() {
            assert_eq!(
                Parameters::default().data(),
                ([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1)
            );
            assert_eq!(
                Parameters::default()
                    .tx_data_rate(DataRate::Kbps424)
                    .rx_data_rate(DataRate::Kbps424)
                    .fdt(FDT::new(1, 2, 3).unwrap())
                    .data(),
                ([0xA0, 0x01, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00], 4),
            );
        }
    }
}

pub mod card_emulation {
    use super::super::ProtocolParams;

    pub type Listen = bool;

    #[derive(Debug, Default)]
    pub struct Parameters {
        wait_for_field: bool,
        clock_from_field: bool,
    }

    impl Parameters {
        pub fn wait_for_field(self) -> Self {
            Self {
                wait_for_field: true,
                ..self
            }
        }

        pub fn clock_from_field(self) -> Self {
            Self {
                clock_from_field: true,
                ..self
            }
        }
    }

    impl ProtocolParams for Parameters {
        fn data(self) -> ([u8; 8], usize) {
            let mut param_byte = 0;

            let wait_for_field_bit = self.wait_for_field as u8;
            param_byte |= wait_for_field_bit << 3;

            let clock_from_field_bit = self.clock_from_field as u8;
            param_byte |= clock_from_field_bit << 1;

            ([param_byte, 0, 0, 0, 0, 0, 0, 0], 1)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum AntiColState {
        Idle = 0x00,
        ReadyA = 0x01,
        Active = 0x04,
        Halt = 0x80,
        ReadyAs = 0x81,
        ActiveAs = 0x84,
    }

    impl TryFrom<u8> for AntiColState {
        type Error = ();

        fn try_from(value: u8) -> Result<Self, Self::Error> {
            match value {
                0x00 => Ok(AntiColState::Idle),
                0x01 => Ok(AntiColState::ReadyA),
                0x04 => Ok(AntiColState::Active),
                0x80 => Ok(AntiColState::Halt),
                0x81 => Ok(AntiColState::ReadyAs),
                0x84 => Ok(AntiColState::ActiveAs),
                _ => Err(()),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        pub fn test_parameters() {
            assert_eq!(
                Parameters::default().data(),
                ([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1)
            );
            assert_eq!(
                Parameters::default().wait_for_field().data(),
                ([0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1)
            );
            assert_eq!(
                Parameters::default().clock_from_field().data(),
                ([0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1)
            );
            assert_eq!(
                Parameters::default()
                    .wait_for_field()
                    .clock_from_field()
                    .data(),
                ([0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1)
            );
        }
    }
}

pub type UID = [u8; 4];
pub type ATQA = u16;
pub type SAK = u8;

/* ------------------------------ */
/* example of data from datasheet */
/* ------------------------------ */

/* Table 16 */
/* NFC Forum Tag Type 4A */
/* NFC Forum Tag Type 1 (Topaz) */
/* NFC Forum Tag Type 2 */

// last byte is Transmission flags:
pub struct TransmissionFlags {
    topaz: bool,
    /// if set ST25R95 will expect 8 signifant bits in the first byte during reception.
    /// In this case, the first byte received is padded with zeros in lsb to complete the
    /// byte, while the last byte received is padded with zeros in msb.
    split_frame: bool,
    append_crc: bool,
    /// if set then the parity bit must be coded inside the data for each byte to be sent
    /// using the send/receive command in transmit mode, and is not decoded by the
    /// ST25R95 in receive mode. In Receive mode, each data byte is accompanied by an
    /// additional byte which encodes the parity: <data byte> <parity byte> <data byte>.
    /// The parity framing mode is compatible with MIFARE® classic requirements. However,
    /// access to authenticated state must be supported by the external secure host which
    /// embeds the MIFARE® classic library.
    parity_frame_mode: bool,
    number_of_significant_bits_in_last_byte: u8,
}

impl From<u8> for TransmissionFlags {
    fn from(value: u8) -> Self {
        Self {
            topaz: value & 0b1000_0000 != 0,
            split_frame: value & 0b0100_0000 != 0,
            append_crc: value & 0b0010_0000 != 0,
            parity_frame_mode: value & 0b0001_0000 != 0,
            number_of_significant_bits_in_last_byte: value & 0b0000_1111,
        }
    }
}

impl From<TransmissionFlags> for u8 {
    fn from(value: TransmissionFlags) -> Self {
        (value.topaz as u8) << 7
            | (value.split_frame as u8) << 6
            | (value.append_crc as u8) << 5
            | (value.parity_frame_mode as u8) << 4
            | value.number_of_significant_bits_in_last_byte
    }
}

// >>> [0x93, 0x70, 0x80, 0x0F, 0x8C, 0x8E, 0x28]
// TransmissionFlags {
//   append_crc: true,
//   number_of_significant_bits_in_last_byte: 8,
//   ..default()
// }

/* NFC Forum Tag Type 2 */
// >>> [0x26, 0x07] REQA
// <<< [0x44, 0x00, 0x28, 0x00] ATQA
// >>> [0x93, 0x20, 0x08] Anti-collision CL1
// <<< [0x88, 0x04, 0xA8, 0xD5, 0xF1, 0x28, 0x00, 0x00] UID CL1

/* NFC Forum Tag Type 1 (Topaz) */
// >>> [0x26, 0x07] REQA
// <<< [0x00, 0x0C, 0x28, 0x00, 0x00] ATQ0 ATQ1

// >>> [0x78, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xA8] RID
// <<< [0x11, 0x48, 0x6E, 0x56, 0x7A, 0x00, 0x3E, 0x45, 0x08, 0x00, 0x00]
//   Header0 Header1 UID0 UID1 UID2 UID3 CRC0 CRC1Signifcantbits indexColbyte IndexColbit

/* anti-collision command / response using a Split frame: */
// >>> [0x93, 0x20, 0x08] Anticol
// <<< [0x88, 0x04, 0x7B, 0x75, 0xB7, 0xB8, 0x02, 0x04] Collision Detected *B8*
// >>> [0x93, 0x45, 0x88, 0x04, 0x0B, 0x45] Anticol Split frame request *45*
// <<< [0x40, 0x74, 0xB3, 0x23, 0x00, 0x00] Spilt frame Answer *23*

/* Examples of data received by send / receive in Parity Framing mode: */
// <<< [0x32, 0x80, 0x34, 0x00, 0x00] meaning: if the ST25R95 received 2 data bytes:
//   - 0x32 with parity = ‘1’ (0x80)
//   - 0x34 with parity = ‘0’ (0x00) in parity framing mode.

/* Table 17 */

// <<< [0x80B30B8DB500, 0x000000]
//      DataReceivedFromTag(IncludingCRCIfPresent) ReceptionFlags

/// To calculate a position of a collision, application has to take index of byte first.
/// Index of bit indicates a position inside this byte.
/// Note that both indexes start from 0 and bit index can be 8, meaning that collision
/// affected parity.
/// TODO The collision information is only present for a bit rate of 106 kbps for
/// transmission and reception. When other bit rates are selected, the two additional
/// bytes are not transmitted.
pub struct ReceptionFlags {
    collision_detected: bool,
    crc_error: bool,
    parity_error: bool,
    number_of_significant_bits_in_first_byte: u8,
    /// Only valid when collision_detected is true
    index_of_the_first_byte_where_collision_was_detected: u8,
    /// Only valid when collision_detected is true
    index_of_the_first_bit_where_collision_was_detected: u8,
}

impl TryFrom<[u8; 3]> for ReceptionFlags {
    type Error = ();

    fn try_from(data: [u8; 3]) -> Result<Self, Self::Error> {
        let collision_detected = data[0] & 0b1000_0000 != 0;
        let crc_error = data[0] & 0b0010_0000 != 0;
        let parity_error = data[0] & 0b0001_0000 != 0;
        let number_of_significant_bits_in_first_byte = data[0] & 0b0000_1111;
        let index_of_the_first_byte_where_collision_was_detected = data[1];
        let index_of_the_first_bit_where_collision_was_detected = data[2] & 0b000_1111;
        if data[0] & 0b1111_1101 == 0 && data[2] & 0b1111_0000 == 0 {
            Ok(ReceptionFlags {
                collision_detected,
                crc_error,
                parity_error,
                number_of_significant_bits_in_first_byte,
                index_of_the_first_byte_where_collision_was_detected,
                index_of_the_first_bit_where_collision_was_detected,
            })
        } else {
            Err(())
        }
    }
}

impl From<ReceptionFlags> for [u8; 3] {
    fn from(lbf: ReceptionFlags) -> Self {
        [
            (lbf.collision_detected as u8) << 7
                | (lbf.crc_error as u8) << 5
                | (lbf.parity_error as u8) << 4
                | lbf.number_of_significant_bits_in_first_byte,
            lbf.index_of_the_first_byte_where_collision_was_detected,
            lbf.index_of_the_first_bit_where_collision_was_detected,
        ]
    }
}

/* Table 23 */

// >>> [0x0400, 0x08]

/* Table 41 */

pub enum Request {
    Sens,                  // 0x26 (7-bits) => REQA
    All,                   // 0x52 (7-bits)
    SingleDeviceDetection, // 0x93 or 0x95 or 0x97
    Select,                // 0x9370 or 0x9570 or 0x9770
    Sleep,                 // 0x5000
}
