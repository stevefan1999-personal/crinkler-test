use alloc::alloc::{GlobalAlloc, Layout};
use winapi::um::heapapi::GetProcessHeap;
use winapi::um::heapapi::HeapAlloc;
use winapi::um::heapapi::HeapFree;
use winapi::um::heapapi::HeapReAlloc;
use winapi::um::winnt::HEAP_ZERO_MEMORY;

pub struct HeapAllocator;

unsafe impl GlobalAlloc for HeapAllocator {
    #[inline(always)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HeapAlloc(GetProcessHeap(), 0, layout.size()) as *mut u8
    }

    #[inline(always)]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        HeapFree(GetProcessHeap(), 0, ptr as *mut winapi::ctypes::c_void);
    }

    #[inline(always)]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, layout.size()) as *mut u8
    }

    #[inline(always)]
    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        HeapReAlloc(
            GetProcessHeap(),
            0,
            ptr as *mut winapi::ctypes::c_void,
            new_size,
        ) as *mut u8
    }
}
