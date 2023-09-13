#![feature(lang_items)]
#![no_std]
#![no_main]
#![allow(internal_features)]
#![windows_subsystem = "console"]

extern crate alloc;

use alloc::vec;
use anyhow::Error;
use core::panic::PanicInfo;
use libc_print::std_name::{eprintln, println};

#[no_mangle]
pub extern "system" fn mainCRTStartup() {
    unsafe {
        winapi::um::processthreadsapi::ExitProcess(match main() {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        });
    }
}

fn main() -> anyhow::Result<()> {
    println!("hello world! {:?}", vec![1, 2, 3, 4]);
    Err("fuck").map_err(Error::msg)
}

#[panic_handler]
#[no_mangle]
pub unsafe extern "C" fn panic(info: &PanicInfo) -> ! {
    eprintln!("{}", info);
    libc::abort()
}

mod quirks {
    #[lang = "eh_personality"]
    #[no_mangle]
    pub extern "C" fn eh_personality() {}

    #[no_mangle]
    pub static _fltused: i32 = 0;
}

mod global_alloc {
    use libc_alloc::LibcAlloc;

    #[global_allocator]
    static ALLOCATOR: LibcAlloc = LibcAlloc;
}
