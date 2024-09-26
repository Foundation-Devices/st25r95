use {
    super::Register,
    crate::{iso15693::Modulation, Protocol, St25r95Error},
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ReceiverGain {
    Db34 = 0x0,
    Db32 = 0x1,
    Db27 = 0x3,
    Db20 = 0x7,
    Db8 = 0xF,
}

/// Adjusting the Modulation Index and Receiver Gain parameters in reader mode can help to
/// improve application behavior.
/// The default values of these parameters are set by the ProtocolSelect command, but they
/// can be overwritten using the WriteRegister command.
#[derive(Debug, Copy, Clone)]
pub struct ArcB {
    pub modulation_index: ModulationIndex,
    pub receiver_gain: ReceiverGain,
}

impl ArcB {
    pub fn new<E: Debug>(
        protocol: Protocol,
        modulation: Option<Modulation>,
        modulation_index: ModulationIndex,
        receiver_gain: ReceiverGain,
    ) -> Result<Self, St25r95Error<E>> {
        // See Table 35
        match protocol {
            Protocol::Iso14443A => {
                if [ModulationIndex::Percent95].contains(&modulation_index) {
                    Ok(Self {
                        modulation_index,
                        receiver_gain,
                    })
                } else {
                    Err(St25r95Error::InvalidModulationIndex {
                        modulation_index,
                        protocol,
                        modulation,
                    })
                }
            }
            Protocol::Iso14443B | Protocol::FeliCa => {
                if [
                    ModulationIndex::Percent10,
                    ModulationIndex::Percent17,
                    ModulationIndex::Percent25,
                    ModulationIndex::Percent30,
                ]
                .contains(&modulation_index)
                {
                    Ok(Self {
                        modulation_index,
                        receiver_gain,
                    })
                } else {
                    Err(St25r95Error::InvalidModulationIndex {
                        modulation_index,
                        protocol,
                        modulation,
                    })
                }
            }
            Protocol::Iso15693 => match modulation {
                None => Err(St25r95Error::NoModulationParameter),
                Some(Modulation::Percent10) => {
                    if [
                        ModulationIndex::Percent30,
                        ModulationIndex::Percent33,
                        ModulationIndex::Percent36,
                    ]
                    .contains(&modulation_index)
                    {
                        Ok(Self {
                            modulation_index,
                            receiver_gain,
                        })
                    } else {
                        Err(St25r95Error::InvalidModulationIndex {
                            modulation_index,
                            protocol,
                            modulation,
                        })
                    }
                }
                Some(Modulation::Percent100) => {
                    if [ModulationIndex::Percent95].contains(&modulation_index) {
                        Ok(Self {
                            modulation_index,
                            receiver_gain,
                        })
                    } else {
                        Err(St25r95Error::InvalidModulationIndex {
                            modulation_index,
                            protocol,
                            modulation,
                        })
                    }
                }
            },
            Protocol::FieldOff | Protocol::CardEmulationIso14443A => {
                Err(St25r95Error::IncompatibleProtocol { protocol })
            }
        }
    }

    pub fn default<E: Debug>(
        protocol: Protocol,
        modulation: &Option<Modulation>,
    ) -> Result<Self, St25r95Error<E>> {
        // See Table 35
        match protocol {
            Protocol::FieldOff | Protocol::CardEmulationIso14443A => {
                Err(St25r95Error::IncompatibleProtocol { protocol })
            }
            Protocol::Iso14443A => Ok(Self {
                modulation_index: ModulationIndex::Percent95,
                receiver_gain: ReceiverGain::Db8,
            }),
            Protocol::Iso14443B => Ok(Self {
                modulation_index: ModulationIndex::Percent17,
                receiver_gain: ReceiverGain::Db34,
            }),
            Protocol::FeliCa => Ok(Self {
                modulation_index: ModulationIndex::Percent33,
                receiver_gain: ReceiverGain::Db34,
            }),
            Protocol::Iso15693 => match modulation {
                None => Err(St25r95Error::NoModulationParameter),
                Some(Modulation::Percent10) => Ok(Self {
                    modulation_index: ModulationIndex::Percent33,
                    receiver_gain: ReceiverGain::Db27,
                }),
                Some(Modulation::Percent100) => Ok(Self {
                    modulation_index: ModulationIndex::Percent95,
                    receiver_gain: ReceiverGain::Db27,
                }),
            },
        }
    }
}

impl Register for ArcB {
    fn control(&self) -> u8 {
        0x68
    }
    fn data(&self) -> [u8; 2] {
        [
            0x01,
            (self.modulation_index as u8) << 4 | (self.receiver_gain as u8),
        ]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Debug)]
    struct TestError {}

    #[test]
    pub fn test_arc_b_data() {
        assert_eq!(
            ArcB::new::<TestError>(
                Protocol::FeliCa,
                None,
                ModulationIndex::Percent17,
                ReceiverGain::Db27
            )
            .unwrap()
            .data(),
            [0x01, 0x23]
        );
        assert_eq!(
            ArcB::default::<TestError>(Protocol::Iso14443A, &None)
                .unwrap()
                .data(),
            [0x01, 0xDF]
        );
        assert_eq!(
            ArcB::default::<TestError>(Protocol::Iso14443B, &None)
                .unwrap()
                .data(),
            [0x01, 0x20]
        );
        assert_eq!(
            ArcB::default::<TestError>(Protocol::FeliCa, &None)
                .unwrap()
                .data(),
            [0x01, 0x50]
        );
        assert_eq!(
            ArcB::default::<TestError>(Protocol::Iso15693, &Some(Modulation::Percent10))
                .unwrap()
                .data(),
            [0x01, 0x53]
        );
        assert_eq!(
            ArcB::default::<TestError>(Protocol::Iso15693, &Some(Modulation::Percent100))
                .unwrap()
                .data(),
            [0x01, 0xD3]
        );
    }
}
