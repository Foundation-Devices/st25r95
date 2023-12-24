// SPDX-FileCopyrightText: 2023 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#[derive(Debug, Copy, Clone)]
pub(crate) enum Command {
    Idn = 0x01,
    ProtocolSelect = 0x02,
    PollField = 0x03,
    SendRecv = 0x04,
    Listen = 0x05,
    Send = 0x06,
    WrReg = 0x09,
    Echo = 0x55,
}

#[derive(Debug, Copy, Clone)]
pub enum PollParams {
    NoParams,
    WaitForField { presc: u8, timer: u8 },
}
