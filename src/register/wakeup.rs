use super::Register;

#[derive(Debug, Copy, Clone)]
pub(crate) struct Wakeup;

impl Register for Wakeup {
    fn read_addr(&self) -> u8 {
        0x62
    }
    fn write_addr(&self) -> u8 {
        unreachable!("Wakeup register is read-only")
    }
    fn index_confirmation(&self) -> u8 {
        unreachable!("Wakeup register is read-only")
    }
    fn has_index(&self) -> bool {
        false
    }
    fn value(&self) -> u8 {
        unreachable!("Wakeup register is read-only")
    }
}
