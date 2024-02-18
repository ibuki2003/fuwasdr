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
static mut ROT_COUNT: i32 = 0;

static mut PIN_BTN1: Option<PinButton1> = None;
static mut PIN_BTN2: Option<PinButton2> = None;
// each bit represents a button
static mut BTN_PRESSED: u8 = 0;

pub fn init(
    pin_rot_a: PinRotaryA,
    pin_rot_b: PinRotaryB,
    pin_btn1: PinButton1,
    pin_btn2: PinButton2,
) {
    unsafe {
        pin_rot_a.set_interrupt_enabled(EdgeLow, true);
        pin_rot_a.set_interrupt_enabled(EdgeHigh, true);
        pin_rot_b.set_interrupt_enabled(EdgeLow, true);
        pin_rot_b.set_interrupt_enabled(EdgeHigh, true);

        pin_btn1.set_interrupt_enabled(EdgeLow, true);
        pin_btn1.set_interrupt_enabled(EdgeHigh, true);
        pin_btn2.set_interrupt_enabled(EdgeLow, true);
        pin_btn2.set_interrupt_enabled(EdgeHigh, true);

        PIN_ROT_A.replace(pin_rot_a);
        PIN_ROT_B.replace(pin_rot_b);

        PIN_BTN1.replace(pin_btn1);
        PIN_BTN2.replace(pin_btn2);

        pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
    }
}

// fetch update since last call
// returns (rot_count, buttons)
pub fn fetch_inputs() -> (i32, u8) {
    critical_section::with(|_| unsafe {
        let count = ROT_COUNT;
        let btns = BTN_PRESSED;
        ROT_COUNT = 0;
        BTN_PRESSED = 0;
        (count, btns)
    })
}

// this handler must not be called before init
#[allow(non_snake_case)]
#[interrupt]
fn IO_IRQ_BANK0() {
    let pin_rot_a = unsafe { PIN_ROT_A.as_mut().unwrap() };
    let pin_rot_b = unsafe { PIN_ROT_B.as_mut().unwrap() };
    static mut ROT_STATE: u8 = 3;
    let rot_state = unsafe { &mut ROT_STATE };

    let pin_btn1 = unsafe { PIN_BTN1.as_mut().unwrap() };
    let pin_btn2 = unsafe { PIN_BTN2.as_mut().unwrap() };
    static mut BTN1_LAST: u32 = 0;
    static mut BTN2_LAST: u32 = 0;
    let btn1_last = unsafe { &mut BTN1_LAST };
    let btn2_last = unsafe { &mut BTN2_LAST };

    let now = unsafe { &*pac::TIMER::PTR }.timerawl.read().bits();
    let now = if now == 0 { 1 } else { now };

    if pin_btn1.is_high().unwrap() {
        if *btn1_last != 0 && now.wrapping_sub(*btn1_last) > 1000 {
            unsafe { BTN_PRESSED |= 1 };
        }
        *btn1_last = 0
    } else if *btn1_last == 0 {
        *btn1_last = now
    }

    if pin_btn2.is_high().unwrap() {
        if *btn2_last != 0 && now.wrapping_sub(*btn2_last) > 1000 {
            unsafe { BTN_PRESSED |= 2 };
        }
        *btn2_last = 0
    } else if *btn2_last == 0 {
        *btn2_last = now
    }

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

    pin_rot_a.clear_interrupt(EdgeLow);
    pin_rot_a.clear_interrupt(EdgeHigh);
    pin_rot_b.clear_interrupt(EdgeLow);
    pin_rot_b.clear_interrupt(EdgeHigh);
    pin_btn1.clear_interrupt(EdgeLow);
    pin_btn1.clear_interrupt(EdgeHigh);
    pin_btn2.clear_interrupt(EdgeLow);
    pin_btn2.clear_interrupt(EdgeHigh);
}
