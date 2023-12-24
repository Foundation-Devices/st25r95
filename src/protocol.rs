// SPDX-FileCopyrightText: 2023 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

// See datasheet table 11
#[derive(Debug, Eq, PartialEq)]
pub enum Protocol {
    /// ISO/IEC 15693
    Iso15693 = 0x01,

    /// ISO/IEC 14443-A
    Iso14443A = 0x02,

    /// ISO/IEC 14443-B
    Iso14443B = 0x03,

    /// FeliCa
    FeliCa = 0x04,

    /// Card Emulation with ISO/IEC 14443-A
    CardEmulationIso14443A = 0x12,
}

pub struct ProtocolSelection {
    pub(crate) protocol: Protocol,
    pub(crate) parameters: [u8; 8],
    pub(crate) param_len: usize,
}

#[derive(Debug, Default)]
pub enum Iso15693Speed {
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
pub struct Iso15693ProtocolBuilder {
    speed: Iso15693Speed,
    wait_for_sof: bool,
    modulation: Modulation,
    subcarrier: Subcarrier,
    with_crc: bool,
}

impl Iso15693ProtocolBuilder {
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

    pub fn speed(self, speed: Iso15693Speed) -> Self {
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

#[derive(Debug, Default)]
pub struct CardEmulationIso14443AProtocolBuilder {
    wait_for_field: bool,
    clock_from_field: bool,
}

impl CardEmulationIso14443AProtocolBuilder {
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

#[derive(Debug, Copy, Clone, Default)]
pub enum Iso14443ADataRate {
    #[default]
    Kbps106 = 0b00,
    Kbps212 = 0b01,
    Kbps424 = 0b10,
}

#[derive(Debug, Default)]
pub struct Iso14443AProtocolBuilder {
    tx_data_rate: Iso14443ADataRate,
    rx_data_rate: Iso14443ADataRate,
    fdt: Option<(u8, u8, u8)>,
}

impl Iso14443AProtocolBuilder {
    pub fn build(self) -> ProtocolSelection {
        let mut parameters = [0; 8];
        let mut param_byte = 0;
        let tx_data_rate_bits = self.tx_data_rate as u8;
        param_byte |= tx_data_rate_bits << 6;
        let rx_data_rate_bits = self.rx_data_rate as u8;
        param_byte |= rx_data_rate_bits << 4;

        parameters[0] = param_byte;
        let param_len = if let Some((pp, mm, dd)) = self.fdt {
            parameters[1] = pp;
            parameters[2] = mm;
            parameters[3] = dd;
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

    pub fn tx_data_rate(self, tx_data_rate: Iso14443ADataRate) -> Self {
        Self {
            tx_data_rate,
            ..self
        }
    }

    pub fn rx_data_rate(self, rx_data_rate: Iso14443ADataRate) -> Self {
        Self {
            rx_data_rate,
            ..self
        }
    }

    pub fn fdt(self, pp: u8, mm: u8, dd: u8) -> Self {
        Self {
            fdt: Some((pp, mm, dd)),
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
        assert_parameters(
            Protocol::Iso15693,
            Iso15693ProtocolBuilder::default().build(),
            &[0x00],
        );

        // H 100 S + crc
        assert_parameters(
            Protocol::Iso15693,
            Iso15693ProtocolBuilder::default().with_crc().build(),
            &[0x01],
        );

        // H 100 D + crc
        assert_parameters(
            Protocol::Iso15693,
            Iso15693ProtocolBuilder::default()
                .subcarrier(Subcarrier::Double)
                .with_crc()
                .build(),
            &[0x03],
        );

        // H 10 S + crc
        assert_parameters(
            Protocol::Iso15693,
            Iso15693ProtocolBuilder::default()
                .modulation(Modulation::Modulation10Percent)
                .with_crc()
                .build(),
            &[0x05],
        );

        // H 10 D + crc
        assert_parameters(
            Protocol::Iso15693,
            Iso15693ProtocolBuilder::default()
                .modulation(Modulation::Modulation10Percent)
                .subcarrier(Subcarrier::Double)
                .with_crc()
                .build(),
            &[0x07],
        );

        // H 10 D - crc
        assert_parameters(
            Protocol::Iso15693,
            Iso15693ProtocolBuilder::default()
                .modulation(Modulation::Modulation10Percent)
                .subcarrier(Subcarrier::Double)
                .build(),
            &[0x06],
        );

        // L 100 S + crc
        assert_parameters(
            Protocol::Iso15693,
            Iso15693ProtocolBuilder::default()
                .speed(Iso15693Speed::Speed6KbpsL)
                .with_crc()
                .build(),
            &[0x21],
        );

        // L 10 S + crc
        assert_parameters(
            Protocol::Iso15693,
            Iso15693ProtocolBuilder::default()
                .speed(Iso15693Speed::Speed6KbpsL)
                .modulation(Modulation::Modulation10Percent)
                .with_crc()
                .build(),
            &[0x25],
        );

        // L 10 D + crc
        assert_parameters(
            Protocol::Iso15693,
            Iso15693ProtocolBuilder::default()
                .speed(Iso15693Speed::Speed6KbpsL)
                .modulation(Modulation::Modulation10Percent)
                .subcarrier(Subcarrier::Double)
                .with_crc()
                .build(),
            &[0x27],
        );

        // L 10 D - crc
        assert_parameters(
            Protocol::Iso15693,
            Iso15693ProtocolBuilder::default()
                .speed(Iso15693Speed::Speed6KbpsL)
                .modulation(Modulation::Modulation10Percent)
                .subcarrier(Subcarrier::Double)
                .build(),
            &[0x26],
        );
    }

    #[test]
    pub fn test_card_emulation_iso14443a_builder() {
        assert_parameters(
            Protocol::CardEmulationIso14443A,
            CardEmulationIso14443AProtocolBuilder::default().build(),
            &[0x00],
        );
        assert_parameters(
            Protocol::CardEmulationIso14443A,
            CardEmulationIso14443AProtocolBuilder::default()
                .wait_for_field()
                .build(),
            &[0x08],
        );
        assert_parameters(
            Protocol::CardEmulationIso14443A,
            CardEmulationIso14443AProtocolBuilder::default()
                .clock_from_field()
                .build(),
            &[0x02],
        );
        assert_parameters(
            Protocol::CardEmulationIso14443A,
            CardEmulationIso14443AProtocolBuilder::default()
                .wait_for_field()
                .clock_from_field()
                .build(),
            &[0x0A],
        );
    }

    #[test]
    pub fn test_iso14443a_builder() {
        assert_parameters(
            Protocol::Iso14443A,
            Iso14443AProtocolBuilder::default().build(),
            &[0x00],
        );
        assert_parameters(
            Protocol::Iso14443A,
            Iso14443AProtocolBuilder::default()
                .tx_data_rate(Iso14443ADataRate::Kbps424)
                .rx_data_rate(Iso14443ADataRate::Kbps424)
                .fdt(1, 2, 3)
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
