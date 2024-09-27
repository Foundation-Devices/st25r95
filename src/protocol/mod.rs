// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

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

pub(crate) trait ProtocolParams {
    fn data(self) -> ([u8; 8], usize);
}

pub(crate) struct FieldOff;
impl ProtocolParams for FieldOff {
    fn data(self) -> ([u8; 8], usize) {
        ([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], 1)
    }
}
