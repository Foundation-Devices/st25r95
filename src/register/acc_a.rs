// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use {super::Register, crate::Error, core::fmt::Debug};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LoadModulationIndex(u8);

impl LoadModulationIndex {
    pub fn min() -> Self {
        Self(0x1)
    }

    pub fn max() -> Self {
        Self(0xf)
    }
}

impl Default for LoadModulationIndex {
    fn default() -> Self {
        Self(0x7)
    }
}

impl TryFrom<u8> for LoadModulationIndex {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if !(0x1..=0xf).contains(&value) {
            return Err(());
        }
        Ok(LoadModulationIndex(value))
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DemodulatorSensitivity {
    Percent10 = 0x1,
    Percent100 = 0x2,
}

impl TryFrom<u8> for DemodulatorSensitivity {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x1 => Ok(Self::Percent10),
            0x2 => Ok(Self::Percent100),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AccA {
    pub load_modulation_index: LoadModulationIndex,
    pub demodulator_sensitivity: DemodulatorSensitivity,
}

impl Register for AccA {
    fn read_addr(&self) -> u8 {
        0x69
    }
    fn write_addr(&self) -> u8 {
        0x68
    }
    fn index_confirmation(&self) -> u8 {
        0x04
    }
    fn has_index(&self) -> bool {
        true
    }
    fn value(&self) -> u8 {
        (self.demodulator_sensitivity as u8) << 4 | self.load_modulation_index.0
    }
}

impl AccA {
    pub(crate) fn from_u8<SPI, I, O>(data: u8) -> Result<Self, Error<SPI, I, O>> {
        let load_modulation_index = data & 0xf;
        let load_modulation_index =
            load_modulation_index
                .try_into()
                .map_err(|_| Error::InvalidLoadModulationIndex {
                    load_modulation_index,
                    min: 0x1,
                    max: 0xf,
                })?;
        let demodulator_sensitivity = (data >> 4) & 0b11;
        let demodulator_sensitivity = demodulator_sensitivity
            .try_into()
            .map_err(|_| Error::InvalidDemodulatorSensitivity(demodulator_sensitivity))?;
        let rfu = data >> 6;
        match rfu {
            0 => Ok(Self {
                load_modulation_index,
                demodulator_sensitivity,
            }),
            _ => Err(Error::InvalidRFU(rfu)),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Debug, PartialEq)]
    struct SPI;
    #[derive(Debug, PartialEq)]
    struct I;
    #[derive(Debug, PartialEq)]
    struct O;

    #[test]
    pub fn test_acc_a_from_u8() {
        assert_eq!(
            AccA::from_u8::<SPI, I, O>(0x17),
            Ok(AccA {
                load_modulation_index: LoadModulationIndex::default(),
                demodulator_sensitivity: DemodulatorSensitivity::Percent10
            })
        );
        assert_eq!(
            AccA::from_u8::<SPI, I, O>(0x07),
            Err(Error::InvalidDemodulatorSensitivity(0x0))
        );
        assert_eq!(
            AccA::from_u8::<SPI, I, O>(0x37),
            Err(Error::InvalidDemodulatorSensitivity(0x3))
        );
        assert_eq!(
            AccA::from_u8::<SPI, I, O>(0x20),
            Err(Error::InvalidLoadModulationIndex {
                load_modulation_index: 0x0,
                min: 0x1,
                max: 0xf,
            })
        );
        assert_eq!(
            AccA::from_u8::<SPI, I, O>(0xE7),
            Err(Error::InvalidRFU(0x3))
        );
    }
}
