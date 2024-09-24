use super::{Protocol, ProtocolSelection};

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
pub struct Builder {
    tx_data_rate: DataRate,
    rx_data_rate: DataRate,
    with_crc: bool,
    rwt: Option<RWT>,
}

impl Builder {
    pub fn build(self) -> ProtocolSelection {
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

        ProtocolSelection {
            protocol: Protocol::FeliCa,
            parameters,
            param_len,
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_felica_rwt() {
        assert!(RWT::new(16, 0).is_none());
        assert_eq!(RWT::new(0, 0).unwrap().us(), 302.06488);
        assert_eq!(RWT::new(15, 0).unwrap().us(), 9898062.0);
        assert_eq!(RWT::new(0, 255).unwrap().us(), 77328.61);
    }

    #[test]
    pub fn test_felica_builder() {
        assert_parameters(Protocol::FeliCa, Builder::default().build(), &[0x00, 0x10]);
        assert_parameters(
            Protocol::FeliCa,
            Builder::default().with_crc().build(),
            &[0x01, 0x10],
        );
        assert_parameters(
            Protocol::FeliCa,
            Builder::default()
                .tx_data_rate(DataRate::Kbps212)
                .rx_data_rate(DataRate::Kbps212)
                .with_crc()
                .rwt(RWT::new(1, 2).unwrap())
                .build(),
            &[0x51, 0x10, 0x01, 0x02],
        );
    }

    fn assert_parameters(
        protocol: Protocol,
        protocol_selection: ProtocolSelection,
        expected: &[u8],
    ) {
        assert_eq!(protocol_selection.protocol, protocol);
        assert_eq!(protocol_selection.param_len, expected.len());
        assert_eq!(
            protocol_selection.parameters[..protocol_selection.param_len],
            *expected
        );
    }
}
