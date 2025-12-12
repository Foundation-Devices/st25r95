// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ST25R95 ARC_B Register (Analog RF Control B)
//!
//! This module provides access to the ARC_B register which controls key RF
//! transmission and reception parameters. The ARC_B register is critical for
//! optimizing communication performance with different NFC protocols and tag types.
//!
//! ## Register Overview
//!
//! The ARC_B register controls:
//! - **Modulation Index**: Depth of amplitude modulation for transmission
//! - **Receiver Gain**: Amplification of received signals
//!
//! ## Protocol-Specific Settings
//!
//! Each NFC protocol has optimal ARC_B configurations:
//!
//! ### ISO15693
//! - **10% modulation**: Standard compatibility with most tags
//! - **100% modulation**: Long range, less compatible (represented as 95%)
//! - **27dB gain**: Balanced sensitivity and range
//!
//! ### ISO14443A  
//! - **95% modulation**: Required for Type A compatibility
//! - **8dB gain**: Standard for close-range communication
//!
//! ### ISO14443B
//! - **10-30% modulation**: Flexible options for Type B tags
//! - **34dB gain**: Higher gain for Type B sensitivity
//!
//! ### FeliCa
//! - **10-30% modulation**: Compatible with FeliCa requirements
//! - **34dB gain**: Optimized for FeliCa communication
//!
//! ## Configuration Guidelines
//!
//! ### Modulation Index Selection
//!
//! - **Higher modulation**: Better range, more power consumption
//! - **Lower modulation**: Better compatibility, less power
//! - **Protocol requirements**: Some protocols require specific values
//!
//! ### Receiver Gain Selection
//!
//! - **Higher gain**: Better sensitivity, more noise susceptibility
//! - **Lower gain**: Better noise immunity, reduced range
//! - **Environment**: Noisy environments may need lower gain
//!
//! ## Usage Examples
//!
//! ```rust,ignore
//! // Use protocol-specific default configuration
//! let arc_b = reader.default_arc_b();
//! reader.write_arc_b(arc_b)?;
//!
//! // Custom configuration for specific environment
//! let arc_b = reader.new_arc_b(
//!     ModulationIndex::Percent17,  // Moderate modulation
//!     ReceiverGain::Db20           // Reduced gain for noisy environment
//! )?;
//! reader.write_arc_b(arc_b)?;
//! ```

use {
    super::Register,
    crate::{Error, Result},
    core::fmt::Debug,
};

/// Modulation index configuration for RF transmission
///
/// The modulation index determines the depth of amplitude modulation applied
/// to the RF carrier during transmission. This affects communication range,
/// power consumption, and compatibility with different tag types.
///
/// ## Modulation Index Effects
///
/// **Higher Modulation (95%)**:
/// - **Pros**: Longer communication range, better signal strength
/// - **Cons**: Higher power consumption, potential over-modulation
/// - **Use Cases**: ISO14443A (required), long-range applications
///
/// **Lower Modulation (10-30%)**:
/// - **Pros**: Lower power consumption, better compatibility
/// - **Cons**: Shorter range, weaker signal
/// - **Use Cases**: ISO15693, ISO14443B, FeliCa, power-sensitive applications
///
/// ## Protocol Compatibility
///
/// - **ISO15693**: 10% or 95% (100%) modulation supported
/// - **ISO14443A**: 95% modulation required by standard
/// - **ISO14443B**: 10%, 17%, 25%, 30% modulation supported
/// - **FeliCa**: 10%, 17%, 25%, 30% modulation supported
///
/// ## Selection Guidelines
///
/// 1. **Check protocol requirements** - some protocols require specific values
/// 2. **Consider communication range** - higher modulation for longer range
/// 3. **Evaluate power constraints** - lower modulation for battery-powered devices
/// 4. **Test with actual tags** - real-world compatibility may vary
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ModulationIndex {
    /// 10% modulation index
    ///
    /// Minimal modulation for low-power, short-range communication.
    /// Compatible with ISO15693, ISO14443B, and FeliCa protocols.
    Percent10 = 0x1,

    /// 17% modulation index
    ///
    /// Low-to-moderate modulation for balanced performance.
    /// Supported by ISO14443B and FeliCa protocols.
    Percent17 = 0x2,

    /// 25% modulation index
    ///
    /// Moderate modulation for medium-range communication.
    /// Supported by ISO14443B and FeliCa protocols.
    Percent25 = 0x3,

    /// 30% modulation index
    ///
    /// Higher modulation for improved range and reliability.
    /// Supported by ISO14443B and FeliCa protocols.
    Percent30 = 0x4,

    /// 33% modulation index
    ///
    /// High modulation for long-range ISO15693 communication.
    /// Optimized for 10% modulation ISO15693 mode.
    Percent33 = 0x5,

    /// 36% modulation index
    ///
    /// High modulation for extended ISO15693 range.
    /// Alternative high-power option for ISO15693.
    Percent36 = 0x6,

    /// 95% modulation index
    ///
    /// Maximum modulation for ISO14443A protocol compliance.
    /// Required by ISO14443A standard for Type A communication.
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

/// Receiver gain configuration for RF signal amplification
///
/// The receiver gain determines how much the incoming RF signals from tags
/// are amplified before processing. This affects communication range,
/// sensitivity, and noise immunity.
///
/// ## Gain Level Effects
///
/// **High Gain (34dB)**:
/// - **Pros**: Maximum communication range, best sensitivity
/// - **Cons**: Higher noise susceptibility, potential interference
/// - **Use Cases**: Long-range ISO15693, ISO14443B/FeliCa, quiet environments
///
/// **Medium Gain (27dB, 32dB)**:
/// - **Pros**: Balanced range and noise immunity
/// - **Cons**: Moderate performance in all aspects
/// - **Use Cases**: Standard ISO15693, general-purpose applications
///
/// **Low Gain (20dB, 8dB)**:
/// - **Pros**: Best noise immunity, stable communication
/// - **Cons**: Reduced range, lower sensitivity
/// - **Use Cases**: Noisy environments, close-range communication, ISO14443A
///
/// ## Selection Guidelines
///
/// 1. **Start with protocol default** - each protocol has recommended settings
/// 2. **Consider environment** - noisy areas may need lower gain
/// 3. **Test communication reliability** - verify with actual tags
/// 4. **Adjust for tag distance** - closer tags can use lower gain
///
/// ## Environmental Considerations
///
/// - **Noisy environments** (RF interference, metal): Use lower gain
/// - **Clean environments** (open space, minimal interference): Can use higher gain
/// - **Multi-reader deployments**: Lower gain to reduce interference
/// - **Critical applications**: Use conservative gain for reliability
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ReceiverGain {
    /// 34dB receiver gain (maximum)
    ///
    /// Highest sensitivity and maximum communication range.
    /// Best suited for long-range protocols and quiet environments.
    ///
    /// **Use with**: ISO15693 long-range, ISO14443B, FeliCa
    Db34 = 0x0,

    /// 32dB receiver gain (high)
    ///
    /// High sensitivity with slightly better noise immunity than 34dB.
    /// Good compromise between range and stability.
    ///
    /// **Use with**: ISO15693 medium-range, ISO14443B, FeliCa
    Db32 = 0x1,

    /// 27dB receiver gain (medium-high)
    ///
    /// Balanced performance with good range and moderate noise immunity.
    /// Default choice for many ISO15693 applications.
    ///
    /// **Use with**: ISO15693 standard, general-purpose
    Db27 = 0x3,

    /// 20dB receiver gain (medium-low)
    ///
    /// Reduced gain for improved noise immunity in challenging environments.
    /// Maintains reasonable range while rejecting interference.
    ///
    /// **Use with**: Noisy environments, medium-range applications
    Db20 = 0x7,

    /// 8dB receiver gain (minimum)
    ///
    /// Lowest gain with maximum noise immunity.
    /// Designed for close-range communication in electrically noisy environments.
    ///
    /// **Use with**: ISO14443A, very close range, high-noise environments
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
