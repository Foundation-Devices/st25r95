// SPDX-FileCopyrightText: 2025 Foundation Devices, Inc. <hello@foundation.xyz>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::Result;

pub trait St25r95Gpio {
    fn irq_in_pulse_low(&mut self);
    fn wait_irq_out_falling_edge(&mut self, timeout: u32) -> Result<()>;
}
