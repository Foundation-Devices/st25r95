// SPDX-FileCopyrightText: 2023 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt::Debug;

pub trait Callbacks {
    type Error: Debug;

    fn transfer(&self, write: &[u8], read: &mut [u8]) -> Result<(), Self::Error>;
    fn read(&self, read: &mut [u8]) -> Result<(), Self::Error>;
    fn set_irq_in(&self, high: bool);
    fn select(&self);
    fn release(&self);
    fn delay_ms(&self, ms: u8);
}
