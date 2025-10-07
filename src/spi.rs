// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{Command, PollFlags, ReadResponse, Result};

pub trait St25r95Spi {
    fn poll(&mut self, flags: PollFlags) -> Result<()>;
    fn reset(&mut self) -> Result<()>;
    fn send_command(&mut self, cmd: Command, data: &[u8]) -> Result<()>;
    fn read_data(&mut self) -> Result<ReadResponse>;
    fn read_echo(&mut self) -> Result<()>;
    fn flush(&mut self, skip_cs: bool) -> Result<()>;
}
