use super::{Protocol, ProtocolSelection};

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
pub struct Builder {
    tx_data_rate: DataRate,
    rx_data_rate: DataRate,
    fdt: Option<FDT>,
}

impl Builder {
    pub fn build(self) -> ProtocolSelection {
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

        ProtocolSelection {
            protocol: Protocol::Iso14443A,
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

    pub fn fdt(self, fdt: FDT) -> Self {
        Self {
            fdt: Some(fdt),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_iso14443a_fdt() {
        assert!(FDT::new(16, 0, 0).is_none());
        assert!(FDT::new(0, 0, 128).is_none());
        assert_eq!(FDT::new(0, 0, 0).unwrap().us(), 302.06488);
        assert_eq!(FDT::new(15, 0, 0).unwrap().us(), 9898062.0);
        assert_eq!(FDT::new(0, 255, 0).unwrap().us(), 77328.61);
        assert_eq!(FDT::new(0, 0, 127).unwrap().us(), 601.7699);
    }

    #[test]
    pub fn test_iso14443a_builder() {
        assert_parameters(Protocol::Iso14443A, Builder::default().build(), &[0x00]);
        assert_parameters(
            Protocol::Iso14443A,
            Builder::default()
                .tx_data_rate(DataRate::Kbps424)
                .rx_data_rate(DataRate::Kbps424)
                .fdt(FDT::new(1, 2, 3).unwrap())
                .build(),
            &[0xA0, 0x01, 0x02, 0x03],
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
