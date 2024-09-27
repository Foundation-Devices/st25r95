// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Register;

#[derive(Debug)]
pub(crate) struct AutoDetectFilter;

impl Register for AutoDetectFilter {
    fn read_addr(&self) -> u8 {
        unreachable!("AutoDetectFilter register is write-only")
    }
    fn write_addr(&self) -> u8 {
        0x0A
    }
    fn index_confirmation(&self) -> u8 {
        0xA1
    }
    fn has_index(&self) -> bool {
        false
    }
    fn value(&self) -> u8 {
        0x02
    }
}
