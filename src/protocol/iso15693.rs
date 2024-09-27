pub mod reader {
    use super::super::ProtocolParams;

    #[derive(Debug, Default)]
    pub enum Speed {
        Kbps52 = 0b01,
        #[default]
        Kbps26H = 0b00,
        Kbps6L = 0b10,
    }

    #[derive(Debug, Default, Clone, Copy)]
    pub enum Modulation {
        #[default]
        Percent100 = 0,
        Percent10 = 1,
    }

    #[derive(Debug, Default)]
    pub enum Subcarrier {
        #[default]
        Single = 0,
        Double = 1,
    }

    #[derive(Debug, Default)]
    pub struct Parameters {
        speed: Speed,
        wait_for_sof: bool,
        modulation: Modulation,
        subcarrier: Subcarrier,
        with_crc: bool,
    }

    impl Parameters {
        pub fn speed(self, speed: Speed) -> Self {
            Self { speed, ..self }
        }

        pub fn wait_for_sof(self) -> Self {
            Self {
                wait_for_sof: true,
                ..self
            }
        }

        pub fn modulation(self, modulation: Modulation) -> Self {
            Self { modulation, ..self }
        }

        pub(crate) fn get_modulation(&self) -> Modulation {
            self.modulation
        }

        pub fn subcarrier(self, subcarrier: Subcarrier) -> Self {
            Self { subcarrier, ..self }
        }

        pub fn with_crc(self) -> Self {
            Self {
                with_crc: true,
                ..self
            }
        }
    }

    impl ProtocolParams for Parameters {
        fn data(self) -> ([u8; 8], usize) {
            let mut param_byte = 0x00;

            let speed_bits = self.speed as u8;
            param_byte |= speed_bits << 4;

            let wait_for_sof_bit = self.wait_for_sof as u8;
            param_byte |= wait_for_sof_bit << 3;

            let modulation_bit = matches!(self.modulation, Modulation::Percent10) as u8;
            param_byte |= modulation_bit << 2;

            let subcarrier_bit = matches!(self.subcarrier, Subcarrier::Double) as u8;
            param_byte |= subcarrier_bit << 1;

            let crc_bit = self.with_crc as u8;
            param_byte |= crc_bit;

            ([param_byte, 0, 0, 0, 0, 0, 0, 0], 1)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        pub fn test_parameters() {
            // H 100 S - crc
            assert_eq!(
                Parameters::default().data(),
                ([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1),
            );

            // H 100 S + crc
            assert_eq!(
                Parameters::default().with_crc().data(),
                ([0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1),
            );

            // H 100 D + crc
            assert_eq!(
                Parameters::default()
                    .subcarrier(Subcarrier::Double)
                    .with_crc()
                    .data(),
                ([0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1),
            );

            // H 10 S + crc
            assert_eq!(
                Parameters::default()
                    .modulation(Modulation::Percent10)
                    .with_crc()
                    .data(),
                ([0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1),
            );

            // H 10 D + crc
            assert_eq!(
                Parameters::default()
                    .modulation(Modulation::Percent10)
                    .subcarrier(Subcarrier::Double)
                    .with_crc()
                    .data(),
                ([0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1),
            );

            // H 10 D - crc
            assert_eq!(
                Parameters::default()
                    .modulation(Modulation::Percent10)
                    .subcarrier(Subcarrier::Double)
                    .data(),
                ([0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1),
            );

            // L 100 S + crc
            assert_eq!(
                Parameters::default().speed(Speed::Kbps6L).with_crc().data(),
                ([0x21, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1),
            );

            // L 10 S + crc
            assert_eq!(
                Parameters::default()
                    .speed(Speed::Kbps6L)
                    .modulation(Modulation::Percent10)
                    .with_crc()
                    .data(),
                ([0x25, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1),
            );

            // L 10 D + crc
            assert_eq!(
                Parameters::default()
                    .speed(Speed::Kbps6L)
                    .modulation(Modulation::Percent10)
                    .subcarrier(Subcarrier::Double)
                    .with_crc()
                    .data(),
                ([0x27, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1),
            );

            // L 10 D - crc
            assert_eq!(
                Parameters::default()
                    .speed(Speed::Kbps6L)
                    .modulation(Modulation::Percent10)
                    .subcarrier(Subcarrier::Double)
                    .data(),
                ([0x26, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1),
            );
        }
    }
}

/* ------------------------------ */
/* example of data from datasheet */
/* ------------------------------ */

/* NFC Forum Tag Type 5 */

/* Table 16 */

// >>> [0x02, 0x20, 0x00] ???

// Inventory command using different protocol configuration:
// Uplink: 100% ASK, 1/4 coding
// Downlink: High data rate, Single sub-carrier
// >>> [0x26, 0x01, 0x00] (Inventory - 1 slot)
// <<< [0x00, 0x00, 0xCD, 0xE0, 0x40, 0x6C, 0xD6, 0x29, 0x02, 0xE0, 0x0579, 0x00]

// >>> [] only the EOF will be sent. This can be used for an anti-collision procedure.

/* Table 17 */

// This is a response to Read Single Block command
// <<< [0x0000000000, 0x77CF, 0x00] DataFromTag Original(Received)ValueOfCRC
// ReceptionFlags

pub struct ReceptionFlags {
    crc_error: bool,
    collision_detected: bool,
}

impl TryFrom<u8> for ReceptionFlags {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let crc_error = value & 0b0000_0010 != 0;
        let collision_detected = value & 0b0000_0001 != 0;
        if value & 0b1111_1100 == 0 {
            Ok(ReceptionFlags {
                crc_error,
                collision_detected,
            })
        } else {
            Err(())
        }
    }
}

impl From<ReceptionFlags> for u8 {
    fn from(lbf: ReceptionFlags) -> Self {
        (lbf.crc_error as u8) << 1 | (lbf.collision_detected as u8)
    }
}
