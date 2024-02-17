use crate::hal;

hal::bsp_pins! {
    Gpio0  { name: i2c_sda, aliases: { FunctionI2c, PullUp: PinI2cSda  } },
    Gpio1  { name: i2c_scl, aliases: { FunctionI2c, PullUp: PinI2cScl } },

    Gpio2  { name: codec_mclk, aliases: { FunctionPio0, PullUp: PinCodecMclk   } },
    Gpio3  { name: codec_bclk, aliases: { FunctionPio0, PullUp: PinCodecBclk   } },
    Gpio4  { name: codec_wclk, aliases: { FunctionPio0, PullUp: PinCodecWclk   } },
    Gpio5  { name: codec_mfp1, aliases: { FunctionPio0, PullUp: PinCodecMfp1   } },
    Gpio6  { name: codec_mfp2, aliases: { FunctionPio0, PullUp: PinCodecMfp2   } },
    Gpio7  { name: codec_mfp3, aliases: { FunctionNull, PullNone: PinCodecMfp3 } },
    Gpio8  { name: codec_mfp4, aliases: { FunctionNull, PullNone: PinCodecMfp4 } },
    Gpio9  { name: codec_mfp5, aliases: { FunctionNull, PullNone: PinCodecMfp5 } },

    Gpio10 { name: sd_sck,  aliases: { FunctionNull, PullNone: PinSdSck  } },
    Gpio11 { name: sd_mosi, aliases: { FunctionNull, PullNone: PinSdMosi } },
    Gpio12 { name: sd_miso, aliases: { FunctionNull, PullNone: PinSdMiso } },
    Gpio13 { name: sd_cs,  aliases: { FunctionNull, PullNone: PinSdCs   } },

    Gpio14 { name: lcd_reset,    aliases: { FunctionSioOutput, PullUp: PinLcdReset    } },
    Gpio16 { name: lcd_miso,     aliases: { FunctionSpi,       PullUp: PinLcdMiso     } },
    Gpio18 { name: lcd_sck,      aliases: { FunctionSpi,       PullUp: PinLcdSck      } },
    Gpio19 { name: lcd_mosi,     aliases: { FunctionSpi,       PullUp: PinLcdMosi     } },
    Gpio15 { name: lcd_touchirq, aliases: { FunctionSioInput,  PullUp: PinLcdTouchIrq } },
    Gpio17 { name: lcd_touchcs,  aliases: { FunctionSioOutput, PullUp: PinLcdTouchCs  } },
    Gpio20 { name: lcd_dispcs,   aliases: { FunctionSioOutput, PullUp: PinLcdDispCs   } },
    Gpio21 { name: lcd_dcrs,     aliases: { FunctionSioOutput, PullUp: PinLcdDcRs     } },

    Gpio22 { name: rotary_a, aliases: { FunctionSioInput, PullUp: PinRotaryA } },
    Gpio26 { name: rotary_b, aliases: { FunctionSioInput, PullUp: PinRotaryB } },

    Gpio23 { name: bpowersave, aliases: { FunctionNull, PullNone: BPowerSave     } },
    Gpio24 { name: vbus, aliases: { FunctionNull, PullNone: VBusDetect     } },
    Gpio25 { name: led, aliases: { FunctionSioOutput, PullNone: OnBoardLed     } },

    Gpio27 { name: button_2, aliases: { FunctionSioInput, PullUp: PinButton2     } },
    Gpio28 { name: button_1, aliases: { FunctionSioInput, PullUp: PinButton1     } },
}

pub type I2CDevice = hal::pac::I2C0;
pub type I2C = hal::i2c::I2C<I2CDevice, (PinI2cSda, PinI2cScl)>;
pub type LcdSpiDevice = hal::pac::SPI0;
