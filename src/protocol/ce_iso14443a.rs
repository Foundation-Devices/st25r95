use super::ProtocolParams;

#[derive(Debug, Default)]
pub struct Parameters {
    wait_for_field: bool,
    clock_from_field: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_card_emulation_iso14443a_parameters() {
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
