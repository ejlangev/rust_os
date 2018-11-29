
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(test, allow(unused_imports))]

use core::panic::PanicInfo;

mod vga_buffer;

use crate::vga_buffer::DebugLevel;

// This is the function that is called during a panic
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
  println!("{}", info);
  loop {}
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn _start() -> ! {
  debug!(DebugLevel::Core, "Hello {}\nGoodbye {}", 23, 24);
  debug!(DebugLevel::Process, "Hello {}\nGoodbye {}", 25, 26);
  println!("Other messages");

  clear_screen!();

  debug!(DebugLevel::Core, "Starting boot...");
  loop {}
}
