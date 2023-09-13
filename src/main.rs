#![feature(lang_items, inline_const, error_in_core, lazy_cell)]
#![no_std]
#![no_main]
#![allow(internal_features)]
#![windows_subsystem = "console"]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::ffi::CString;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use anyhow::bail;
use core::cell::LazyCell;
use core::ffi::CStr;
use core::mem::{self, MaybeUninit};
use core::panic::PanicInfo;
use core::ptr;
use global_alloc::HeapAllocator;
use libc::c_void;
use libc_print::std_name::{eprintln, println};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, Win32WindowHandle};
use vulkanalia::loader::Loader;
use vulkanalia::vk::{EntryV1_0, ExtDebugUtilsExtension, ExtensionName, HasBuilder};
use vulkanalia::{vk, Entry};
use winapi::shared::minwindef::{HINSTANCE, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::libloaderapi::{GetModuleHandleA, GetProcAddress, LoadLibraryA};
use winapi::um::processthreadsapi::ExitProcess;
use winapi::um::winuser::{
    BeginPaint, DefWindowProcA, DispatchMessageA, DrawTextA, EndPaint, GetClientRect, GetMessageA,
    InvalidateRect, PostQuitMessage, TranslateMessage, DT_CENTER, DT_SINGLELINE, DT_VCENTER,
};
use winapi::um::winuser::{
    CreateWindowExA, RegisterClassA, ShowWindow, CW_USEDEFAULT, SW_SHOW, WNDCLASSA,
    WS_OVERLAPPEDWINDOW,
};

// Assign our wrapper as the global allocator
#[global_allocator]
static __: HeapAllocator = HeapAllocator;

#[doc(hidden)]
pub const fn validate_cstr_contents(bytes: &[u8]) {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\0' {
            panic!("null in cstr");
        }
        i += 1;
    }
}

macro_rules! cstr {
    ( $s:literal ) => {{
        const { $crate::validate_cstr_contents($s.as_bytes()) };
        #[allow(unused_unsafe)]
        unsafe {
            core::ffi::CStr::from_bytes_with_nul_unchecked(concat!($s, "\0").as_bytes())
        }
    }};
}

#[no_mangle]
pub unsafe extern "system" fn mainCRTStartup() {
    // AllocConsole();
    unsafe {
        ExitProcess(match main() {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        });
    }
}

#[derive(Debug)]
struct Window {
    pub window_handle: *const HWND,
    pub instance_handle: *const HINSTANCE,
}

unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut window_handle = Win32WindowHandle::empty();
        window_handle.hwnd = self.window_handle as *mut _;
        window_handle.hinstance = self.instance_handle as *mut _;
        RawWindowHandle::Win32(window_handle)
    }
}

const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

const VALIDATION_LAYER: ExtensionName = ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

fn main() -> anyhow::Result<()> {
    println!("{}", "hello");

    let hInstance = unsafe { GetModuleHandleA(ptr::null()) };

    println!("{:?}", hInstance);

    unsafe {
        RegisterClassA(&WNDCLASSA {
            hInstance,
            lpszClassName: cstr!("hello world!").as_ptr(),
            lpfnWndProc: Some(window_proc),
            ..Default::default()
        })
    };

    let hwnd = unsafe {
        CreateWindowExA(
            0,
            cstr!("hello world!").as_ptr(),
            cstr!("hello world!").as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            ptr::null_mut(),
            ptr::null_mut(),
            hInstance,
            ptr::null_mut(),
        )
    };

    let window = Window {
        instance_handle: hInstance as *const _,
        window_handle: hwnd as *const _,
    };

    unsafe { ShowWindow(hwnd, SW_SHOW) };

    let application_info = vk::ApplicationInfo::builder()
        .application_name(b"Vulkan Tutorial\0")
        .application_version(vk::make_version(1, 0, 0))
        .engine_name(b"No Engine\0")
        .engine_version(vk::make_version(1, 0, 0))
        .api_version(vk::make_version(1, 0, 0));

    let mut extensions = vulkanalia::window::get_required_instance_extensions(&window)
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();

    if VALIDATION_ENABLED {
        extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
    }

    // println!("{:?}", extensions);

    let entry = unsafe { Entry::new(MyLoader) }.map_err(anyhow::Error::msg)?;

    let available_layers = unsafe { entry.enumerate_instance_layer_properties() }
        .map_err(anyhow::Error::msg)?
        .iter()
        .map(|l| l.layer_name)
        .collect::<BTreeSet<_>>();

    if VALIDATION_ENABLED && available_layers.contains(&VALIDATION_LAYER) {
        bail!("Validation layer requested but not supported.");
    }

    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        Vec::new()
    };

    let info = vk::InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions);

    let instance = unsafe { entry.create_instance(&info, None) }.map_err(anyhow::Error::msg)?;

    let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
        .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
        .user_callback(Some(debug_callback));

    unsafe {
        instance
            .create_debug_utils_messenger_ext(&debug_info, None)
            .map_err(anyhow::Error::msg)
    }?;

    println!("{:?}", instance);

    loop {
        let mut msg = MaybeUninit::uninit();
        unsafe {
            if GetMessageA(msg.as_mut_ptr(), hwnd, 0, 0) > 0 {
                TranslateMessage(msg.as_ptr());
                DispatchMessageA(msg.as_ptr());
            } else {
                break;
            }
        }
    }

    Ok(())
}

extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        println!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        println!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        println!("({:?}) {}", type_, message);
    } else {
        println!("({:?}) {}", type_, message);
    }

    vk::FALSE
}

static mut COUNTER: usize = 0;

pub unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        winapi::um::winuser::WM_LBUTTONDOWN => {
            COUNTER += 1;
            InvalidateRect(hwnd, ptr::null(), true.into());
        }
        winapi::um::winuser::WM_PAINT => {
            let text = CString::new(COUNTER.to_string()).unwrap();

            let mut paint_struct = MaybeUninit::uninit();
            let mut rect = MaybeUninit::uninit();
            let hdc = BeginPaint(hwnd, paint_struct.as_mut_ptr());
            GetClientRect(hwnd, rect.as_mut_ptr());
            DrawTextA(
                hdc,
                text.as_ptr(),
                -1,
                rect.as_mut_ptr(),
                DT_SINGLELINE | DT_CENTER | DT_VCENTER,
            );
            EndPaint(hwnd, paint_struct.as_mut_ptr());
        }
        winapi::um::winuser::WM_DESTROY => {
            PostQuitMessage(0);
        }
        _ => {
            return DefWindowProcA(hwnd, msg, wparam, lparam);
        }
    }
    return 0;
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

mod global_alloc;

struct MyLoader;

static mut VULKAN: LazyCell<HINSTANCE> =
    LazyCell::new(|| unsafe { LoadLibraryA(cstr!("vulkan-1.dll").as_ptr()) });

impl Loader for MyLoader {
    unsafe fn load(
        &self,
        name: &[u8],
    ) -> Result<extern "system" fn(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        if *VULKAN != ptr::null_mut() {
            let name = CString::new(name)?;
            let symbol = GetProcAddress(*VULKAN, name.as_ptr() as _);
            if symbol != ptr::null_mut() {
                Ok(mem::transmute(symbol))
            } else {
                Err("unable to get symbol".into())
            }
        } else {
            Err("unable to load vulkan library".into())
        }
    }
}
