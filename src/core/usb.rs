use crate::{ hal,  SAMPLE_RATE, core::dma::{DMA_IDX, DMABUF}};
use hal:: pac::{self, interrupt} ;

pub static mut USBDEV: Option<UsbDev<'static>> = None;

pub struct UsbDev<'a> {
    usb_dev: usb_device::prelude::UsbDevice<'a, hal::usb::UsbBus>,
    usb_audio: usbd_audio::AudioClass<'a, hal::usb::UsbBus>,
}

impl UsbDev<'static> {
    pub fn init(
        usb_bus: &'static usb_device::bus::UsbBusAllocator<hal::usb::UsbBus>,
    ) {
        let usb_audio = usbd_audio::AudioClassBuilder::new()
            .input(
                usbd_audio::StreamConfig::new_discrete(
                    usbd_audio::Format::S16le,
                    2,
                    &[SAMPLE_RATE as u32],
                    usbd_audio::TerminalType::InMicrophone,
                )
                .unwrap(),
            )
            .build(usb_bus)
            .unwrap();

        let usb_dev = usb_device::prelude::UsbDeviceBuilder::new(
            usb_bus,
            usb_device::prelude::UsbVidPid(0x16c0, 0x27dd),
        )
        .max_packet_size_0(64)
        .manufacturer("Fake company")
        .product("Audio")
        .serial_number("TEST")
        .build();


        let u = Self {
            usb_dev,
            usb_audio,
        };

        unsafe {
            USBDEV.replace(u);
            pac::NVIC::unmask(pac::Interrupt::USBCTRL_IRQ);
        }
    }
}


#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    const USBBUF_LEN: usize = 192;
    static mut USBBUF: [u8; USBBUF_LEN * 4] = [0; USBBUF_LEN * 4];
    let usb_buf = unsafe { &mut USBBUF };

    // usb
    let usb = unsafe { USBDEV.as_mut().unwrap() };
    let usb_dev = &mut usb.usb_dev;
    let usb_audio = &mut usb.usb_audio;

    if usb_dev.poll(&mut [usb_audio]) {
        let src_idx = if unsafe { DMA_IDX } < USBBUF_LEN { USBBUF_LEN } else { 0 };
        let src = unsafe { &DMABUF[src_idx..src_idx + USBBUF_LEN] };
        unsafe {core::slice::from_raw_parts_mut(usb_buf.as_mut_ptr() as *mut u32, USBBUF_LEN)}
        .copy_from_slice(src);
        usb_audio.write(usb_buf).ok();
    }
}
