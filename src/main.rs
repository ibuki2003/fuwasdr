#![no_std]
#![no_main]

use fuwasdr::bsp::entry;

#[entry]
fn main() -> ! {
    fuwasdr::core::main()
}
