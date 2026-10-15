use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::Mutex;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub enum NDIlib_FourCC_video_type_e {
    RGBA = 0x4147_524F, // 'RGBA'
    BGRA = 0x4152_4742,
    UYVY = 0x5956_5955,
}

#[repr(C)]
pub struct NDIlib_send_create_t {
    pub p_ndi_name: *const c_char,
    pub p_groups: *const c_char,
    pub clock_video: bool,
    pub clock_audio: bool,
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct NDIlib_video_frame_v2_t {
    pub xres: i32,
    pub yres: i32,
    pub FourCC: NDIlib_FourCC_video_type_e,
    pub frame_rate_N: i32,
    pub frame_rate_D: i32,
    pub picture_aspect_ratio: f32,
    pub frame_format_type: i32,
    pub timecode: i64,
    pub p_data: *const u8,
    pub line_stride_in_bytes: i32,
    pub metadata: *const c_char,
    pub timestamp: i64,
}

type NdilibInitializeFn = unsafe extern "C" fn() -> bool;
type NdilibSendCreateFn = unsafe extern "C" fn(p_create_settings: *const NDIlib_send_create_t) -> *mut std::ffi::c_void;
type NdilibSendSendVideoV2Fn = unsafe extern "C" fn(p_instance: *mut std::ffi::c_void, p_video_data: *const NDIlib_video_frame_v2_t);
type NdilibSendDestroyFn = unsafe extern "C" fn(p_instance: *mut std::ffi::c_void);

pub struct NdiSenderEngine {
    _lib: Option<Library>,
    sender: Mutex<Option<*mut std::ffi::c_void>>,
    fn_send_video: Option<NdilibSendSendVideoV2Fn>,
    fn_destroy: Option<NdilibSendDestroyFn>,
    name: Mutex<String>,
    enabled: Mutex<bool>,
}

unsafe impl Send for NdiSenderEngine {}
unsafe impl Sync for NdiSenderEngine {}

impl NdiSenderEngine {
    pub fn new(initial_name: &str) -> Self {
        let dll_path = "NDI/Processing.NDI.Lib.x64.dll";
        let loaded_lib = match unsafe { Library::new(dll_path) } {
            Ok(lib) => {
                println!("[NDI Sender] Successfully loaded native NDI DLL from: {}", dll_path);
                Some(lib)
            }
            Err(e) => {
                eprintln!("[NDI Sender] Error loading NDI DLL from {}: {}", dll_path, e);
                None
            }
        };

        let mut sender_ptr = None;
        let mut fn_send_video = None;
        let mut fn_destroy = None;

        if let Some(ref lib) = loaded_lib {
            unsafe {
                let init_res: Result<Symbol<NdilibInitializeFn>, _> = lib.get(b"NDIlib_initialize\0");
                if let Ok(init_fn) = init_res {
                    if init_fn() {
                        println!("[NDI Sender] NDI SDK Initialized successfully.");
                    }
                }

                let create_res: Result<Symbol<NdilibSendCreateFn>, _> = lib.get(b"NDIlib_send_create\0");
                if let Ok(create_fn) = create_res {
                    let c_name = CString::new(initial_name).unwrap();
                    let settings = NDIlib_send_create_t {
                        p_ndi_name: c_name.as_ptr(),
                        p_groups: std::ptr::null(),
                        clock_video: true,
                        clock_audio: false,
                    };
                    let instance = create_fn(&settings);
                    if !instance.is_null() {
                        sender_ptr = Some(instance);
                        println!("[NDI Sender] Stream created: '{}'", initial_name);
                    }
                }

                let send_res: Result<Symbol<NdilibSendSendVideoV2Fn>, _> = lib.get(b"NDIlib_send_send_video_v2\0");
                if let Ok(send_fn) = send_res {
                    fn_send_video = Some(*send_fn);
                }

                let destroy_res: Result<Symbol<NdilibSendDestroyFn>, _> = lib.get(b"NDIlib_send_destroy\0");
                if let Ok(destroy_fn) = destroy_res {
                    fn_destroy = Some(*destroy_fn);
                }
            }
        }

        Self {
            _lib: loaded_lib,
            sender: Mutex::new(sender_ptr),
            fn_send_video,
            fn_destroy,
            name: Mutex::new(initial_name.to_string()),
            enabled: Mutex::new(true),
        }
    }

    pub fn set_enabled(&self, is_enabled: bool) {
        let mut guard = self.enabled.lock().unwrap();
        *guard = is_enabled;
        println!("[NDI Sender] Streaming enabled state: {}", is_enabled);
    }

    pub fn set_name(&self, new_name: &str) {
        let mut name_guard = self.name.lock().unwrap();
        *name_guard = new_name.to_string();
    }

    pub fn send_rgba_frame(&self, width: u32, height: u32, rgba_data: &[u8]) {
        let enabled = *self.enabled.lock().unwrap();
        if !enabled {
            return;
        }

        let sender_guard = self.sender.lock().unwrap();
        if let (Some(sender), Some(send_fn)) = (*sender_guard, self.fn_send_video) {
            let frame = NDIlib_video_frame_v2_t {
                xres: width as i32,
                yres: height as i32,
                FourCC: NDIlib_FourCC_video_type_e::RGBA,
                frame_rate_N: 60000,
                frame_rate_D: 1000,
                picture_aspect_ratio: width as f32 / height as f32,
                frame_format_type: 0,
                timecode: 0,
                p_data: rgba_data.as_ptr(),
                line_stride_in_bytes: (width * 4) as i32,
                metadata: std::ptr::null(),
                timestamp: 0,
            };

            unsafe {
                send_fn(sender, &frame);
            }
        }
    }
}

impl Drop for NdiSenderEngine {
    fn drop(&mut self) {
        let mut sender_guard = self.sender.lock().unwrap();
        if let (Some(sender), Some(destroy_fn)) = (sender_guard.take(), self.fn_destroy) {
            unsafe {
                destroy_fn(sender);
            }
        }
    }
}
