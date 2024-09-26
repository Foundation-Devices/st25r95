// SPDX-FileCopyrightText: 2023 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod acc_a;
pub mod arc_b;
pub mod auto_detect_filter;
pub mod timer_window;

pub trait Register {
    fn control(&self) -> u8;
    fn data(&self) -> [u8; 2];
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ReadableRegister {
    AccA,
    ArcB,
    WakeupEvent,
}
