use crate::board::*;
use crate::hal;
use embedded_hal::digital::InputPin;
use hal::{
    gpio::Interrupt::{EdgeHigh, EdgeLow},
    pac::{self, interrupt},
};

// // NOTE: output B is unstable, so we need to use input A for interrupt
static mut PIN_ROT_A: Option<PinRotaryA> = None;
static mut PIN_ROT_B: Option<PinRotaryB> = None;
static mut ROT_STATE: u8 = 3;
static mut ROT_COUNT: i32 = 0;

pub fn init(pin_rot_a: PinRotaryA, pin_rot_b: PinRotaryB) {
    unsafe {
        pin_rot_a.set_interrupt_enabled(EdgeLow, true);
        pin_rot_a.set_interrupt_enabled(EdgeHigh, true);
        pin_rot_b.set_interrupt_enabled(EdgeLow, true);
        pin_rot_b.set_interrupt_enabled(EdgeHigh, true);

        PIN_ROT_A.replace(pin_rot_a);
        PIN_ROT_B.replace(pin_rot_b);

        pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
    }
}

// fetch the count and reset it
pub fn pop_count() -> i32 {
    critical_section::with(|_| unsafe {
        let count = ROT_COUNT;
        ROT_COUNT = 0;
        count
    })
}

// this handler must not be called before init
#[allow(non_snake_case)]
#[interrupt]
fn IO_IRQ_BANK0() {
    let pin_rot_a = unsafe { PIN_ROT_A.as_mut().unwrap() };
    let pin_rot_b = unsafe { PIN_ROT_B.as_mut().unwrap() };
    let rot_state = unsafe { &mut ROT_STATE };

    pin_rot_a.clear_interrupt(EdgeLow);
    pin_rot_a.clear_interrupt(EdgeHigh);
    pin_rot_b.clear_interrupt(EdgeLow);
    pin_rot_b.clear_interrupt(EdgeHigh);

    let state_now =
        (pin_rot_a.is_high().unwrap() as u8) | (pin_rot_b.is_high().unwrap() as u8) << 1;

    if state_now == 0 {
        // found rotation
        if *rot_state == 1 {
            // CCW
            unsafe { ROT_COUNT -= 1 };
        } else if *rot_state == 2 {
            // CW
            unsafe { ROT_COUNT += 1 };
        }
    }

    if state_now == 3 || *rot_state != 0 {
        *rot_state = state_now;
    }
}
