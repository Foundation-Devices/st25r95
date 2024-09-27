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
