use crate::hal;
use hal::gpio::bank0::*;

pub type PinI2cSda = Gpio0;
pub type PinI2cScl = Gpio1;

pub type PinCodecMclk = Gpio2;
pub type PinCodecBclk = Gpio3;
pub type PinCodecWclk = Gpio4;
pub type PinCodecMfp1 = Gpio5;
pub type PinCodecMfp2 = Gpio6;
pub type PinCodecMfp3 = Gpio7;
pub type PinCodecMfp4 = Gpio8;
pub type PinCodecMfp5 = Gpio9;

pub type PinSdSck = Gpio10;
pub type PinSdMosi = Gpio11;
pub type PinSdMiso = Gpio12;
pub type PinSdCs = Gpio13;

// gpio14 unused

pub type PinLcdTouchIrq = Gpio15;
pub type PinLcdMiso = Gpio16;
pub type PinLcdTouchCs = Gpio17;
pub type PinLcdSck = Gpio18;
pub type PinLcdMosi = Gpio19;
pub type PinLcdDispCs = Gpio20;
pub type PinLcdDcRs = Gpio21;

pub type PinRotaryA = Gpio22;
pub type PinRotaryB = Gpio26;

pub type BPowerSave = Gpio23;
pub type VBusDetect = Gpio24;
pub type OnBoardLed = Gpio25;

pub type PinButton2 = Gpio27;
pub type PinButton1 = Gpio28;
