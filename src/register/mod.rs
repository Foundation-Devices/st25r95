// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod acc_a;
pub mod arc_b;
pub mod auto_detect_filter;
pub mod timer_window;
pub mod wakeup;

pub(crate) trait Register {
    fn read_addr(&self) -> u8;
    fn write_addr(&self) -> u8;
    fn index_confirmation(&self) -> u8;
    fn has_index(&self) -> bool;
    fn value(&self) -> u8;
}
