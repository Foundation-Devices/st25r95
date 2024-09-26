use {
    super::Register,
    crate::{Protocol, St25r95Error},
    core::fmt::Debug,
};

/// To improve ST25R95 demodulation when communicating with ISO/IEC 14443 Type A tags, it
/// is possible to adjust the synchronization between digital and analog inputs by
/// fine-tuning the Timer Window value.
/// The default values of these parameters are set by the ProtocolSelect command, but they
/// can be overwritten using the WriteRegister command.
#[derive(Debug, Copy, Clone)]
pub struct TimerWindow(pub(crate) u8);

impl TimerWindow {
    pub fn new<E: Debug>(protocol: Protocol, timer_w: u8) -> Result<Self, St25r95Error<E>> {
        match protocol {
            Protocol::Iso14443A => {
                if (0x50..=0x60).contains(&timer_w) {
                    Ok(Self(timer_w))
                } else {
                    Err(St25r95Error::InvalidU8Parameter {
                        min: 0x50,
                        max: 0x60,
                        actual: timer_w,
                    })
                }
            }
            _ => Err(St25r95Error::IncompatibleProtocol { protocol }),
        }
    }

    pub fn default<E: Debug>(protocol: Protocol) -> Result<Self, St25r95Error<E>> {
        // See §5.11.2
        Self::new(protocol, 0x52)
    }

    pub fn recommended<E: Debug>(protocol: Protocol) -> Result<Self, St25r95Error<E>> {
        // See §5.11.2
        Self::new(protocol, 0x56)
    }
}

impl Register for TimerWindow {
    fn control(&self) -> u8 {
        0x3A
    }
    fn data(&self) -> [u8; 2] {
        [self.0, 0x04]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Debug)]
    struct TestError {}

    #[test]
    pub fn test_timer_window_data() {
        assert_eq!(
            TimerWindow::new::<TestError>(Protocol::Iso14443A, 0x58)
                .unwrap()
                .data(),
            [0x58, 0x04]
        );
        assert_eq!(
            TimerWindow::default::<TestError>(Protocol::Iso14443A)
                .unwrap()
                .data(),
            [0x52, 0x04]
        );
    }
}
