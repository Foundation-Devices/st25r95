use {
    super::Register,
    crate::{Protocol, St25r95Error},
    core::fmt::Debug,
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DemodulatorSensitivity {
    Percent10 = 0x1,
    Percent100 = 0x2,
}

/// Adjusting the Load modulation index and Demodulator sensitivity parameters in card
/// emulation mode can help to improve application behavior.
/// The default values of these parameters are set by the ProtocolSelect command, but they
/// can be overwritten using the WriteRegister command.

#[derive(Debug, Copy, Clone)]
pub struct AccA {
    pub load_modulation_index: u8,
    pub demodulator_sensitivity: DemodulatorSensitivity,
}

impl AccA {
    pub(crate) fn new<E: Debug>(
        protocol: Protocol,
        load_modulation_index: u8,
        demodulator_sensitivity: DemodulatorSensitivity,
    ) -> Result<Self, St25r95Error<E>> {
        // See Table 36
        match protocol {
            Protocol::CardEmulationIso14443A => {
                if demodulator_sensitivity != DemodulatorSensitivity::Percent100 {
                    Err(St25r95Error::InvalidDemodulatorSensitivity {
                        demodulator_sensitivity,
                        protocol,
                    })
                } else if !(0x1..=0xf).contains(&load_modulation_index) {
                    Err(St25r95Error::InvalidLoadModulationIndex {
                        load_modulation_index,
                        min: 0x1,
                        max: 0xf,
                        protocol,
                    })
                } else {
                    Ok(Self {
                        load_modulation_index,
                        demodulator_sensitivity,
                    })
                }
            }
            _ => Err(St25r95Error::IncompatibleProtocol { protocol }),
        }
    }

    pub fn default<E: Debug>(protocol: Protocol) -> Result<Self, St25r95Error<E>> {
        // See Table 36
        Self::new(protocol, 0x7, DemodulatorSensitivity::Percent100)
    }

    pub fn recommended<E: Debug>(protocol: Protocol) -> Result<Self, St25r95Error<E>> {
        // See Table 36
        Self::default(protocol)
    }
}

impl Register for AccA {
    fn control(&self) -> u8 {
        0x68
    }
    fn data(&self) -> [u8; 2] {
        [
            0x04,
            (self.demodulator_sensitivity as u8) << 4 | self.load_modulation_index,
        ]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Debug)]
    struct TestError {}

    #[test]
    pub fn test_acc_a_data() {
        assert_eq!(
            AccA::new::<TestError>(
                Protocol::CardEmulationIso14443A,
                5,
                DemodulatorSensitivity::Percent100
            )
            .unwrap()
            .data(),
            [0x04, 0x25]
        );
        assert_eq!(
            AccA::default::<TestError>(Protocol::CardEmulationIso14443A)
                .unwrap()
                .data(),
            [0x04, 0x27]
        );
    }
}
