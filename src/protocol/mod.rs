// SPDX-FileCopyrightText: 2023 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod ce_iso14443a;
pub mod felica;
pub mod iso14443a;
pub mod iso14443b;
pub mod iso15693;

// See datasheet table 11
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Protocol {
    /// Field OFF
    FieldOff = 0x00,

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

impl ProtocolSelection {
    pub fn field_off() -> Self {
        Self {
            protocol: Protocol::FieldOff,
            parameters: [0; 8],
            param_len: 0,
        }
    }

    // TODO: find a better way to not have to extract this info
    pub(crate) fn modulation(&self) -> Option<iso15693::Modulation> {
        if self.protocol == Protocol::Iso15693 {
            Some(if self.parameters[0] & 0b0000_0100 == 0 {
                iso15693::Modulation::Percent100
            } else {
                iso15693::Modulation::Percent10
            })
        } else {
            None
        }
    }
}
