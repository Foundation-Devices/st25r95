use {
    super::Register,
    crate::{Protocol, St25r95Error},
    core::fmt::Debug,
};

/// To improve ST25R95 reception when communicating with FeliCa™ tags, it is possible to
/// enable an AutoDetect filter to synchronize FeliCa™ tags with the ST25R95. This can be
/// done using the WriteRegister command to enable the AutoDetect filter.
/// By default, this filter is disabled after the execution of the ProtocolSelect command,
/// but it can be enabled using the WritreRegister command
pub struct AutoDetectFilter {}

impl AutoDetectFilter {
    pub fn enable<E: Debug>(protocol: Protocol) -> Result<Self, St25r95Error<E>> {
        match protocol {
            Protocol::FeliCa => Ok(Self {}),
            _ => Err(St25r95Error::IncompatibleProtocol { protocol }),
        }
    }
}

impl Register for AutoDetectFilter {
    fn control(&self) -> u8 {
        0x0A
    }
    fn data(&self) -> [u8; 2] {
        [0x02, 0xA1]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Debug)]
    struct TestError {}

    #[test]
    pub fn test_auto_detect_filter_data() {
        assert_eq!(
            AutoDetectFilter::enable::<TestError>(Protocol::FeliCa)
                .unwrap()
                .data(),
            [0x02, 0xA1]
        );
    }
}
