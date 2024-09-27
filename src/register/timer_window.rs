// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Register;

#[derive(Debug, Copy, Clone)]
pub struct TimerWindow(pub(crate) u8);

impl Register for TimerWindow {
    fn read_addr(&self) -> u8 {
        unreachable!("TimerWindow register is write-only")
    }
    fn write_addr(&self) -> u8 {
        0x3A
    }
    fn index_confirmation(&self) -> u8 {
        0x04
    }
    fn has_index(&self) -> bool {
        false
    }
    fn value(&self) -> u8 {
        self.0
    }
}
