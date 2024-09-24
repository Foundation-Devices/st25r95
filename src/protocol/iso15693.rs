use super::{Protocol, ProtocolSelection};

#[derive(Debug, Default)]
pub enum Speed {
    Speed52Kbps = 0b01,
    #[default]
    Speed26KbpsH = 0b00,
    Speed6KbpsL = 0b10,
}

#[derive(Debug, Default)]
pub enum Modulation {
    #[default]
    Modulation100Percent = 0,
    Modulation10Percent = 1,
}

#[derive(Debug, Default)]
pub enum Subcarrier {
    #[default]
    Single = 0,
    Double = 1,
}

#[derive(Debug, Default)]
pub struct Builder {
    speed: Speed,
    wait_for_sof: bool,
    modulation: Modulation,
    subcarrier: Subcarrier,
    with_crc: bool,
}

impl Builder {
    pub fn build(self) -> ProtocolSelection {
        let param_len = 1;
        let mut param_byte = 0x00;

        let speed_bits = self.speed as u8;
        param_byte |= speed_bits << 4;

        let wait_for_sof_bit = self.wait_for_sof as u8;
        param_byte |= wait_for_sof_bit << 3;

        let modulation_bit = matches!(self.modulation, Modulation::Modulation10Percent) as u8;
        param_byte |= modulation_bit << 2;

        let subcarrier_bit = matches!(self.subcarrier, Subcarrier::Double) as u8;
        param_byte |= subcarrier_bit << 1;

        let crc_bit = self.with_crc as u8;
        param_byte |= crc_bit;

        ProtocolSelection {
            protocol: Protocol::Iso15693,
            param_len,
            parameters: [param_byte, 0, 0, 0, 0, 0, 0, 0],
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_iso15693_builder() {
        // H 100 S - crc
        assert_parameters(Protocol::Iso15693, Builder::default().build(), &[0x00]);

        // H 100 S + crc
        assert_parameters(
            Protocol::Iso15693,
            Builder::default().with_crc().build(),
            &[0x01],
        );

        // H 100 D + crc
        assert_parameters(
            Protocol::Iso15693,
            Builder::default()
                .subcarrier(Subcarrier::Double)
                .with_crc()
                .build(),
            &[0x03],
        );

        // H 10 S + crc
        assert_parameters(
            Protocol::Iso15693,
            Builder::default()
                .modulation(Modulation::Modulation10Percent)
                .with_crc()
                .build(),
            &[0x05],
        );

        // H 10 D + crc
        assert_parameters(
            Protocol::Iso15693,
            Builder::default()
                .modulation(Modulation::Modulation10Percent)
                .subcarrier(Subcarrier::Double)
                .with_crc()
                .build(),
            &[0x07],
        );

        // H 10 D - crc
        assert_parameters(
            Protocol::Iso15693,
            Builder::default()
                .modulation(Modulation::Modulation10Percent)
                .subcarrier(Subcarrier::Double)
                .build(),
            &[0x06],
        );

        // L 100 S + crc
        assert_parameters(
            Protocol::Iso15693,
            Builder::default()
                .speed(Speed::Speed6KbpsL)
                .with_crc()
                .build(),
            &[0x21],
        );

        // L 10 S + crc
        assert_parameters(
            Protocol::Iso15693,
            Builder::default()
                .speed(Speed::Speed6KbpsL)
                .modulation(Modulation::Modulation10Percent)
                .with_crc()
                .build(),
            &[0x25],
        );

        // L 10 D + crc
        assert_parameters(
            Protocol::Iso15693,
            Builder::default()
                .speed(Speed::Speed6KbpsL)
                .modulation(Modulation::Modulation10Percent)
                .subcarrier(Subcarrier::Double)
                .with_crc()
                .build(),
            &[0x27],
        );

        // L 10 D - crc
        assert_parameters(
            Protocol::Iso15693,
            Builder::default()
                .speed(Speed::Speed6KbpsL)
                .modulation(Modulation::Modulation10Percent)
                .subcarrier(Subcarrier::Double)
                .build(),
            &[0x26],
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
