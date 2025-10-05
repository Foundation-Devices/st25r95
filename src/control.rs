// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use bitflags::bitflags;

#[derive(Debug, Copy, Clone)]
pub enum Control {
    Send = 0x00,
    Reset = 0x01,
    Read = 0x02,
    Poll = 0x03,
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct PollFlags: u8 {
        /// Data can be read from the ST25R95 when set.
        const CAN_READ = 1 << 3;

        /// Data can be sent to the ST25R95 when set.
        const CAN_SEND = 1 << 2;
    }
}
