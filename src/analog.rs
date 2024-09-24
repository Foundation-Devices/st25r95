// SPDX-FileCopyrightText: 2023 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use {
    crate::{iso15693::Modulation, Protocol, St25r95Error},
    core::fmt::Debug,
};

#[derive(Debug, Copy, Clone)]
pub enum AnalogParam {
    AccA(AccA),

    ArcB(ArcB),

    /// Timer Window value for the synchronization between digital and analog inputs
    /// during the demodulation of ISO/IEC 14443 Type A tag frames.
    ///
    /// Minimum `0x50`, maximum `0x60`. Default is `0x52`. Recommended is `0x56` or
    /// `0x58`.
    TimerW(TimerW),
}

#[derive(Debug, Copy, Clone)]
pub struct ArcB {
    pub modulation_index: ModulationIndex,
    pub receiver_gain: ReceiverGain,
}

impl ArcB {
    fn validate<E: Debug>(
        &self,
        protocol: Protocol,
        modulation: Option<Modulation>,
    ) -> Result<(), St25r95Error<E>> {
        // See Table 35
        match protocol {
            Protocol::FieldOff => Err(St25r95Error::UnsupportedAnalogParameterValueForProtocol), /* TODO ? */
            Protocol::Iso14443A => {
                if self.modulation_index as u8 != 0x0d {
                    Err(St25r95Error::UnsupportedAnalogParameterValueForProtocol)
                } else {
                    Ok(())
                }
            }
            Protocol::Iso14443B | Protocol::FeliCa => {
                if ![0x1, 0x2, 0x3, 0x4].contains(&(self.modulation_index as u8)) {
                    Err(St25r95Error::UnsupportedAnalogParameterValueForProtocol)
                } else {
                    Ok(())
                }
            }
            Protocol::Iso15693 => match modulation {
                None => Err(St25r95Error::NoModulationParameter),
                Some(Modulation::Modulation10Percent) => {
                    if ![0x04, 0x05, 0x06].contains(&(self.modulation_index as u8)) {
                        Err(St25r95Error::UnsupportedAnalogParameterValueForProtocol)
                    } else {
                        Ok(())
                    }
                }
                Some(Modulation::Modulation100Percent) => {
                    if ![0x0d].contains(&(self.modulation_index as u8)) {
                        Err(St25r95Error::UnsupportedAnalogParameterValueForProtocol)
                    } else {
                        Ok(())
                    }
                }
            },
            Protocol::CardEmulationIso14443A => Err(St25r95Error::UnsupportedProtocolSelected),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AccA {
    pub load_modulation_index: LoadModulationIndex,
    pub demodulator_sensitivity: DemodulatorSensitivity,
}

impl AccA {
    fn validate<E: Debug>(&self, protocol: Protocol) -> Result<(), St25r95Error<E>> {
        // See Table 36
        match protocol {
            Protocol::CardEmulationIso14443A => {
                if self.demodulator_sensitivity as u8 != 0x02 {
                    Err(St25r95Error::UnsupportedAnalogParameterValueForProtocol)
                } else {
                    Ok(())
                }
            }
            _ => Err(St25r95Error::UnsupportedProtocolSelected),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ModulationIndex {
    Percent10 = 0x01,
    Percent17 = 0x02,
    Percent25 = 0x03,
    Percent30 = 0x04,
    Percent33 = 0x05,
    Percent36 = 0x06,
    Percent95 = 0x0D,
}

#[derive(Debug, Copy, Clone)]
pub enum ReceiverGain {
    Db34 = 0x00,
    Db32 = 0x01,
    Db27 = 0x03,
    Db20 = 0x07,
    Db8 = 0x0F,
}

#[derive(Debug, Copy, Clone)]
pub struct LoadModulationIndex(u8);

impl TryFrom<u8> for LoadModulationIndex {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= 0x0f {
            Ok(Self(value))
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DemodulatorSensitivity {
    Percent10 = 0x01,
    Percent100 = 0x02,
}

#[derive(Debug, Copy, Clone)]
pub struct TimerW(u8);

impl TimerW {
    pub fn new(timer_w: u8) -> Self {
        Self(timer_w)
    }

    fn validate<E: Debug>(&self, protocol: Protocol) -> Result<(), St25r95Error<E>> {
        if let Protocol::Iso14443A = protocol {
            if self.0 < 0x50 || self.0 > 0x60 {
                Err(St25r95Error::UnsupportedAnalogParameterValueForProtocol)
            } else {
                Ok(())
            }
        } else {
            Err(St25r95Error::UnsupportedProtocolSelected)
        }
    }
}

impl AnalogParam {
    pub fn try_new_acc_a<E: Debug>(
        protocol: Protocol,
        acc_a: AccA,
    ) -> Result<AnalogParam, St25r95Error<E>> {
        acc_a.validate(protocol)?;
        Ok(AnalogParam::AccA(acc_a))
    }

    pub fn try_new_arc_b<E: Debug>(
        protocol: Protocol,
        modulation: Option<Modulation>,
        arc_b: ArcB,
    ) -> Result<AnalogParam, St25r95Error<E>> {
        arc_b.validate(protocol, modulation)?;
        Ok(AnalogParam::ArcB(arc_b))
    }

    pub fn try_new_timer_w<E: Debug>(
        protocol: Protocol,
        timer_w: TimerW,
    ) -> Result<AnalogParam, St25r95Error<E>> {
        timer_w.validate(protocol)?;
        Ok(AnalogParam::TimerW(timer_w))
    }

    pub(crate) fn as_slice(&self, slice: &mut [u8]) -> usize {
        assert!(slice.len() >= 4);

        match self {
            AnalogParam::AccA(AccA {
                load_modulation_index: LoadModulationIndex(load_modulation_index),
                demodulator_sensitivity,
            }) => {
                let byte = (*demodulator_sensitivity as u8) << 4 | *load_modulation_index;
                let len = 5;
                slice[..len].copy_from_slice(&[0x04, 0x68, 0x01, 0x04, byte]);
                len
            }
            AnalogParam::ArcB(ArcB {
                modulation_index,
                receiver_gain,
            }) => {
                let byte = (*modulation_index as u8) << 4 | (*receiver_gain as u8);
                let len = 5;
                slice[..len].copy_from_slice(&[0x04, 0x68, 0x01, 0x01, byte]);
                len
            }
            AnalogParam::TimerW(TimerW(timer_w)) => {
                let len = 5;
                slice[..len].copy_from_slice(&[0x04, 0x3A, 0x00, *timer_w, 0x04]);
                len
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestError {}

    #[test]
    pub fn test_analog_param() {
        assert_analog_param(
            AnalogParam::try_new_acc_a::<TestError>(
                Protocol::CardEmulationIso14443A,
                AccA {
                    load_modulation_index: LoadModulationIndex::try_from(5).unwrap(),
                    demodulator_sensitivity: DemodulatorSensitivity::Percent100,
                },
            )
            .unwrap(),
            &[0x04, 0x68, 0x01, 0x04, 0x25],
        );
        assert_analog_param(
            AnalogParam::try_new_arc_b::<TestError>(
                Protocol::FeliCa,
                None,
                ArcB {
                    modulation_index: ModulationIndex::Percent17,
                    receiver_gain: ReceiverGain::Db27,
                },
            )
            .unwrap(),
            &[0x04, 0x68, 0x01, 0x01, 0x23],
        );
        assert_analog_param(
            AnalogParam::try_new_timer_w::<TestError>(Protocol::Iso14443A, TimerW::new(0x58))
                .unwrap(),
            &[0x04, 0x3A, 0x00, 0x58, 0x04],
        );
    }

    fn assert_analog_param(param: AnalogParam, data: &[u8]) {
        let mut buf = [0u8; 5];
        let len = param.as_slice(&mut buf);

        assert_eq!(data.len(), len, "data length doesn't match");
        assert_eq!(
            data[..len],
            buf[..len],
            "analog register data doesn't match the example"
        );
    }
}
