use super::{Protocol, ProtocolSelection};

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
pub struct Builder {
    tx_data_rate: DataRate,
    rx_data_rate: DataRate,
    with_crc: bool,
    fwt: Option<FWT>,
    tttt: u16,
    yy: u8,
    zz: u8,
}

impl Default for Builder {
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

        ProtocolSelection {
            protocol: Protocol::Iso14443B,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_iso14443b_fwt() {
        assert!(FWT::new(16, 0, 0).is_none());
        assert!(FWT::new(0, 0, 128).is_none());
        assert_eq!(FWT::new(0, 0, 0).unwrap().us(), 302.06488);
        assert_eq!(FWT::new(15, 0, 0).unwrap().us(), 9898062.0);
        assert_eq!(FWT::new(0, 255, 0).unwrap().us(), 77328.61);
        assert_eq!(FWT::new(0, 0, 127).unwrap().us(), 601.7699);
    }

    #[test]
    pub fn test_iso14443b_builder() {
        assert_parameters(Protocol::Iso14443B, Builder::default().build(), &[0x00]);
        assert_parameters(
            Protocol::Iso14443B,
            Builder::default().with_crc().build(),
            &[0x01],
        );
        assert_parameters(
            Protocol::Iso14443B,
            Builder::default()
                .tx_data_rate(DataRate::Kbps424)
                .rx_data_rate(DataRate::Kbps424)
                .fwt(FWT::new(1, 2, 3).unwrap())
                .build(),
            &[0xA0, 0x01, 0x02, 0x03, 0x03, 0xFF, 0x00, 0x1A],
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
