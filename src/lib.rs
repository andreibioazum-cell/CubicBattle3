mod lobby;
mod settings;

use android_activity::{AndroidApp, MainEvent, PollEvent};
use glow::HasContext;
use std::ffi::{c_void, CString};
use std::ptr;

// ================= EGL =================

#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(display_id: *mut c_void) -> *mut c_void;
    fn eglInitialize(dpy: *mut c_void, major: *mut i32, minor: *mut i32) -> i32;
    fn eglChooseConfig(
        dpy: *mut c_void,
        attrib_list: *const i32,
        configs: *mut *mut c_void,
        config_size: i32,
        num_config: *mut i32,
    ) -> i32;
    fn eglCreateWindowSurface(
        dpy: *mut c_void,
        config: *mut c_void,
        win: *mut c_void,
        attrib_list: *const i32,
    ) -> *mut c_void;
    fn eglCreateContext(
        dpy: *mut c_void,
        config: *mut c_void,
        share_context: *mut c_void,
        attrib_list: *const i32,
    ) -> *mut c_void;
    fn eglMakeCurrent(
        dpy: *mut c_void,
        draw: *mut c_void,
        read: *mut c_void,
        ctx: *mut c_void,
    ) -> i32;
    fn eglSwapInterval(dpy: *mut c_void, interval: i32) -> i32;
    fn eglSwapBuffers(dpy: *mut c_void, surface: *mut c_void) -> i32;
    fn eglGetProcAddress(procname: *const std::os::raw::c_char) -> *mut c_void;
}

// ================= SCENES =================

enum Scene {
    Lobby,
    Settings,
}

#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default());

    let mut running = true;
    let mut scene = Scene::Lobby;

    let mut display: *mut c_void = ptr::null_mut();
    let mut surface: *mut c_void = ptr::null_mut();
    let mut gl: Option<glow::Context> = None;

    while running {
        app.poll_events(None, |event| {
            match event {
                PollEvent::Main(MainEvent::InitWindow { .. }) => unsafe {
                    let window = app.native_window().unwrap();

                    display = eglGetDisplay(ptr::null_mut());
                    eglInitialize(display, ptr::null_mut(), ptr::null_mut());

                    // ES 3.0 config
                    let config_attribs = [
                        0x3024, 8,  // EGL_RED_SIZE
                        0x3023, 8,  // EGL_GREEN_SIZE
                        0x3022, 8,  // EGL_BLUE_SIZE
                        0x3033, 4,  // EGL_RENDERABLE_TYPE = EGL_OPENGL_ES2_BIT
                        0x3038      // EGL_NONE
                    ];

                    let mut config: *mut c_void = ptr::null_mut();
                    let mut num_config = 0;

                    eglChooseConfig(
                        display,
                        config_attribs.as_ptr(),
                        &mut config,
                        1,
                        &mut num_config,
                    );

                    surface = eglCreateWindowSurface(
                        display,
                        config,
                        window.ptr().as_ptr() as *mut _,
                        ptr::null(),
                    );

                    // ES 3.0 context
                    let context_attribs = [
                        0x3098, 3,  // EGL_CONTEXT_CLIENT_VERSION = 3
                        0x3038
                    ];

                    let context = eglCreateContext(
                        display,
                        config,
                        ptr::null_mut(),
                        context_attribs.as_ptr(),
                    );

                    eglMakeCurrent(display, surface, surface, context);
                    eglSwapInterval(display, 1); // VSYNC ✅

                    let gl_ctx = glow::Context::from_loader_function(|s| {
                        eglGetProcAddress(CString::new(s).unwrap().as_ptr())
                    });

                    gl = Some(gl_ctx);
                },

                PollEvent::Main(MainEvent::Destroy) => {
                    running = false;
                }

                _ => {}
            }
        });

        if let Some(gl) = &gl {
            unsafe {
                gl.viewport(0, 0, 1280, 720);

                match scene {
                    Scene::Lobby => {
                        if lobby::render(gl) {
                            scene = Scene::Settings;
                        }
                    }
                    Scene::Settings => {
                        if settings::render(gl) {
                            scene = Scene::Lobby;
                        }
                    }
                }

                eglSwapBuffers(display, surface);
            }
        }
    }
            }
