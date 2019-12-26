#![allow(dead_code)]

use std::{mem, ptr, os::raw::{c_char, c_void}};
use winapi::{
    shared::{minwindef::*, windef::*},
    um::{libloaderapi::*, wingdi::*, winnt::*, winuser::*},
};
use wstr::{wstrz, wstrz_impl};

// TODO: This might not work on MinGW GCC? It seems to be a "microsoft linker" thing.
extern "C" {
    /// Static handle to the base image. Linker magic.
    /// Healthy, reliable alternative to GetModuleHandle(NULL) always for the *base* image.
    static __ImageBase: IMAGE_DOS_HEADER;
}

/// The pixel format used for the OpenGL drawing context.
// Reading this, you might wonder, "why are the colours 0 bits"?
// How Windows works is that it finds the closest pixel format to this big parameter dump.
// So those pixels won't actually have zero bits, that's just how you do it, it's really ugly.
static PIXEL_FORMAT: PIXELFORMATDESCRIPTOR = PIXELFORMATDESCRIPTOR {
    nSize: mem::size_of::<PIXELFORMATDESCRIPTOR>() as _,
    nVersion: 1,
    dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
    iPixelType: PFD_TYPE_RGBA,
    cColorBits: 32,
    cRedBits: 0,
    cRedShift: 0,
    cGreenBits: 0,
    cGreenShift: 0,
    cBlueBits: 0,
    cBlueShift: 0,
    cAlphaBits: 0,
    cAlphaShift: 0,
    cAccumBits: 0,
    cAccumRedBits: 0,
    cAccumGreenBits: 0,
    cAccumBlueBits: 0,
    cAccumAlphaBits: 0,
    cDepthBits: 24,
    cStencilBits: 8,
    cAuxBuffers: 0,
    iLayerType: PFD_MAIN_PLANE,
    bReserved: 0,
    dwLayerMask: 0,
    dwVisibleMask: 0,
    dwDamageMask: 0,
};

static mut WINDOW_CLASS: Option<ATOM> = None;
static WINDOW_CLASS_NAME: &[u16] = wstrz!("MiniGL");
const WINDOW_STYLE: DWORD = WS_OVERLAPPEDWINDOW | WS_VISIBLE;

/// Registers the window class, if not already registered.
#[rustfmt::skip]
unsafe fn register_window_class() {
    if let None = WINDOW_CLASS {
        let wnd_class = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as _,         // winapi garbage (struct size)
            style: CS_OWNDC,                                    // per-window device contexts
            lpfnWndProc: Some(wnd_proc),                        // window event callback
            cbClsExtra: 0,                                      // extra after-struct alloc
            cbWndExtra: 0,                                      // ^
            hInstance: (&__ImageBase) as *const _ as *mut _,    // base image pointer
            hIcon: ptr::null_mut(),                             // window icon (TODO, kinda?)
            hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),   // basic "arrow" cursor
            hbrBackground: COLOR_BACKGROUND as _,               // background "brush"
            lpszMenuName: ptr::null(),                          // we have no menu
            lpszClassName: WINDOW_CLASS_NAME.as_ptr(),          // class name
            hIconSm: ptr::null_mut(),                           // small icon (null = "use hIcon")
        };
        let class: ATOM = RegisterClassExW(&wnd_class);
        assert_ne!(class, 0 as ATOM); // TODO
        WINDOW_CLASS = Some(class);
    }
}

/// Creates a window with a given title, width, height and class atom.
/// The class atom should be acquired from register_window_class().
unsafe fn create_window(title: &str, mut width: u32, mut height: u32, class: ATOM) {
    let mut window_name = [0u16; 1024];
    let mut utf16_enc = title.encode_utf16();
    window_name
        .iter_mut()
        .take(1023) // leave space for the null
        .zip(&mut utf16_enc)
        .for_each(|(dst, src)| *dst = src);
    assert_eq!(utf16_enc.next(), None); // TODO

    // TODO: document this cap
    width = width.min(i32::max_value() as u32);
    height = height.min(i32::max_value() as u32);

    let window: HWND = CreateWindowExW(
        WS_EX_LEFT,                           // dwExStyle: DWORD
        class as _,                           // lpClassName: LPCWSTR
        window_name.as_ptr(),                 // lpWindowName: LPCWSTR
        WINDOW_STYLE,                         // dwStyle: DWORD
        CW_USEDEFAULT,                        // x: c_int
        CW_USEDEFAULT,                        // y: c_int
        width as _,                           // nWidth: c_int (CW_USEDEFAULT is -2147483648)
        height as _,                          // nHeight: c_int (read above)
        ptr::null_mut(),                      // hWndParent: HWND
        ptr::null_mut(),                      // hMenu: HMENU
        &__ImageBase as *const _ as *mut _,   // hInstance: HINSTANCE
        ptr::null_mut(),                      // lpParam: LPVOID
    );

    assert!(!window.is_null()); // TODO
}

/// The WndProc of the window. Go look it up on MSDN, please.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let device_ctx = GetDC(hwnd);
            assert!(!device_ctx.is_null()); // TODO
            let format = ChoosePixelFormat(device_ctx, &PIXEL_FORMAT);
            assert_ne!(format, 0); // TODO
            SetPixelFormat(device_ctx, format, &PIXEL_FORMAT);
            let opengl_ctx = wglCreateContext(device_ctx);
            assert!(!opengl_ctx.is_null()); // TODO
            let res = wglMakeCurrent(device_ctx, opengl_ctx);
            assert_eq!(res, TRUE); // TODO
            let version = mem::transmute::<_, extern "C" fn(u32) -> *const c_char>(load_function(b"glGetString\0".as_ptr() as *const c_char))(0x1F02);
            let s = std::ffi::CStr::from_ptr(version as *mut _);
            println!("OpenGL Version String: {}", s.to_string_lossy());
            let asdf = load_function(b"wglCreateContextAttribsARB\0".as_ptr() as *const c_char);
            println!("wglCreateContextAttribsARB: {:?}", asdf);
            wglDeleteContext(opengl_ctx);
            PostQuitMessage(0);
        }
        _ => (),
    }
    DefWindowProcW(hwnd, msg, wparam, lparam) // "Process everything else"
}

/// Loads an OpenGL function pointer.
/// Only works if there is a current OpenGL context.
unsafe fn load_function(name: *const c_char) -> Option<ptr::NonNull<c_void>> {
    let addr = wglGetProcAddress(name);
    match addr as isize {
        // All of these return values mean failure, as much as the docs say it's just NULL.
        // You load some of them like this, but only if wglGetProcAddress failed.
        // Thank you, Microsoft.
        -1 | 0 | 1 | 2 | 3 => {
            let module = LoadLibraryA(b"opengl32.dll\0".as_ptr() as *const c_char);
            let function = GetProcAddress(module, name);
            if !function.is_null() {
                Some(ptr::NonNull::new_unchecked(function as *mut _))
            } else {
                None
            }
        }

        _ => Some(ptr::NonNull::new_unchecked(addr as *mut _))
    }
}

pub fn do_it() {
    unsafe {
        register_window_class();
        create_window("owo, what's this?", 800, 608, WINDOW_CLASS.unwrap());
    }
}
