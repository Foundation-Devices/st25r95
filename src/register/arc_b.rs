// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use {
    super::Register,
    crate::{Error, Result},
    core::fmt::Debug,
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ModulationIndex {
    Percent10 = 0x1,
    Percent17 = 0x2,
    Percent25 = 0x3,
    Percent30 = 0x4,
    Percent33 = 0x5,
    Percent36 = 0x6,
    Percent95 = 0xD,
}

impl TryFrom<u8> for ModulationIndex {
    type Error = ();

    fn try_from(value: u8) -> core::result::Result<Self, Self::Error> {
        match value {
            0x1 => Ok(ModulationIndex::Percent10),
            0x2 => Ok(ModulationIndex::Percent17),
            0x3 => Ok(ModulationIndex::Percent25),
            0x4 => Ok(ModulationIndex::Percent30),
            0x5 => Ok(ModulationIndex::Percent33),
            0x6 => Ok(ModulationIndex::Percent36),
            0xD => Ok(ModulationIndex::Percent95),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ReceiverGain {
    Db34 = 0x0,
    Db32 = 0x1,
    Db27 = 0x3,
    Db20 = 0x7,
    Db8 = 0xF,
}

impl TryFrom<u8> for ReceiverGain {
    type Error = ();

    fn try_from(value: u8) -> core::result::Result<Self, Self::Error> {
        match value {
            0x0 => Ok(ReceiverGain::Db34),
            0x1 => Ok(ReceiverGain::Db32),
            0x3 => Ok(ReceiverGain::Db27),
            0x7 => Ok(ReceiverGain::Db20),
            0xF => Ok(ReceiverGain::Db8),
            _ => Err(()),
        }
    }
}

/// Adjusting the Modulation Index and Receiver Gain parameters in reader mode can help to
/// improve application behavior.
/// The default values of these parameters are set by the ProtocolSelect command, but they
/// can be overwritten using the WriteRegister command.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ArcB {
    pub(crate) modulation_index: ModulationIndex,
    pub(crate) receiver_gain: ReceiverGain,
}

impl Register for ArcB {
    fn read_addr(&self) -> u8 {
        0x69
    }
    fn write_addr(&self) -> u8 {
        0x68
    }
    fn index_confirmation(&self) -> u8 {
        0x01
    }
    fn has_index(&self) -> bool {
        true
    }
    fn value(&self) -> u8 {
        (self.modulation_index as u8) << 4 | (self.receiver_gain as u8)
    }
}

impl ArcB {
    pub(crate) fn from_u8(data: u8) -> Result<Self> {
        let modulation_index = (data >> 4) & 0xf;
        let modulation_index = modulation_index
            .try_into()
            .map_err(|_| Error::InvalidModulationIndex(modulation_index))?;
        let receiver_gain = data & 0xf;
        let receiver_gain = receiver_gain
            .try_into()
            .map_err(|_| Error::InvalidReceiverGain(receiver_gain))?;
        Ok(Self {
            modulation_index,
            receiver_gain,
        })
    }

    pub(crate) fn fake() -> Self {
        Self {
            modulation_index: ModulationIndex::Percent17,
            receiver_gain: ReceiverGain::Db27,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    pub fn test_arc_b_from_u8() {
        assert_eq!(
            ArcB::from_u8(0x23u8),
            Ok(ArcB {
                modulation_index: ModulationIndex::Percent17,
                receiver_gain: ReceiverGain::Db27,
            })
        );
        [0x0, 0x7, 0x8, 0x9, 0xA, 0xB, 0xC, 0xE, 0xF]
            .iter()
            .for_each(|i| {
                assert_eq!(
                    ArcB::from_u8(*i << 4 | 0xf),
                    Err(Error::InvalidModulationIndex(*i))
                );
            });
        [0x4, 0x5, 0x6, 0x8, 0x9, 0xA, 0xB, 0xC, 0xD, 0xE]
            .iter()
            .for_each(|i| {
                assert_eq!(
                    ArcB::from_u8(*i | 0x10),
                    Err(Error::InvalidReceiverGain(*i))
                );
            });
    }
}
