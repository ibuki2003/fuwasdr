use core::cell::RefCell;
use critical_section::Mutex;

pub static SHARED_I2CBUS: Mutex<RefCell<Option<crate::board::I2C>>> =
    Mutex::new(RefCell::new(None));
