use super::{Protocol, ProtocolSelection};

#[derive(Debug, Default)]
pub struct Builder {
    wait_for_field: bool,
    clock_from_field: bool,
}

impl Builder {
    pub fn build(self) -> ProtocolSelection {
        let mut param_byte = 0;

        let wait_for_field_bit = self.wait_for_field as u8;
        param_byte |= wait_for_field_bit << 3;

        let clock_from_field_bit = self.clock_from_field as u8;
        param_byte |= clock_from_field_bit << 1;

        ProtocolSelection {
            protocol: Protocol::CardEmulationIso14443A,
            parameters: [param_byte, 0, 0, 0, 0, 0, 0, 0],
            param_len: 1,
        }
    }

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
    pub fn test_card_emulation_iso14443a_builder() {
        assert_parameters(
            Protocol::CardEmulationIso14443A,
            Builder::default().build(),
            &[0x00],
        );
        assert_parameters(
            Protocol::CardEmulationIso14443A,
            Builder::default().wait_for_field().build(),
            &[0x08],
        );
        assert_parameters(
            Protocol::CardEmulationIso14443A,
            Builder::default().clock_from_field().build(),
            &[0x02],
        );
        assert_parameters(
            Protocol::CardEmulationIso14443A,
            Builder::default()
                .wait_for_field()
                .clock_from_field()
                .build(),
            &[0x0A],
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
