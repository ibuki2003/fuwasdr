use crate::hal;

use core::cell::RefCell;
use critical_section::Mutex;
use hal::gpio::{FunctionI2c, Pin, PullUp};

pub type I2C = hal::i2c::I2C<
    hal::pac::I2C0,
    (
        Pin<crate::board::PinI2cSda, FunctionI2c, PullUp>,
        Pin<crate::board::PinI2cScl, FunctionI2c, PullUp>,
    ),
>;
pub static SHARED_I2CBUS: Mutex<RefCell<Option<I2C>>> = Mutex::new(RefCell::new(None));
