// SPDX-FileCopyrightText: 2023 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use core::fmt::Debug;

#[derive(Debug)]
pub enum St25r95Error<E: Debug> {
    SpiError(E),
    PollTimeout,
    IdentificationError,
    CommunicationError,
    FrameTimeoutOrNoTag,
    InvalidSof,
    RxBufferOverflow,
    InternalBufferOverflow,
    InvalidProtocol,
    InvalidCommandLength,
    NoField,
    FramingError,
    UnknownError(u8),
    EgtTimeout,
    InvalidLength,
    CrcError,
    ReceptionLostWithoutEof,
    UnsupportedProtocolSelected,
    UnsupportedAnalogParameterValueForProtocol,
    NoModulationParameter,
}
