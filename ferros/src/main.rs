#![no_main]
#![no_std]

use core::panic::PanicInfo;

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn main() {
    core::arch::naked_asm!("1:", "   wfe", "   b 1b");
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unimplemented!()
}
