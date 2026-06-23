mod lobby;
mod settings;

use android_activity::{AndroidApp, MainEvent, PollEvent};
use glow::HasContext;
use std::ffi::{c_void, CString};
use std::ptr;

#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(id: *mut c_void) -> *mut c_void;
    fn eglInitialize(d: *mut c_void, ma: *mut i32, mi: *mut i32) -> i32;
    fn eglChooseConfig(d: *const i32, al: *const i32, c: *mut *mut c_void, cs: i32, nc: *mut i32) -> i32;
    fn eglCreateWindowSurface(d: *mut c_void, c: *mut c_void, w: *mut c_void, al: *const i32) -> *mut c_void;
    fn eglCreateContext(d: *mut c_void, c: *mut c_void, sc: *mut c_void, al: *const i32) -> *mut c_void;
    fn eglMakeCurrent(d: *mut c_void, dr: *mut c_void, r: *mut c_void, ctx: *mut c_void) -> i32;
    fn eglSwapInterval(d: *mut c_void, interval: i32) -> i32;
    fn eglSwapBuffers(d: *mut c_void, s: *mut c_void) -> i32;
    fn eglGetProcAddress(n: *const std::os::raw::c_char) -> *mut c_void;
}

enum Scene {
    Lobby,
    Settings,
}

#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default());

    let mut scene = Scene::Lobby;
    let mut running = true;

    let mut gl: Option<glow::Context> = None;
    let mut surface = ptr::null_mut();
    let mut display = ptr::null_mut();

    while running {
        app.poll_events(None, |event| {
            match event {
                PollEvent::Main(MainEvent::InitWindow { .. }) => unsafe {
                    let win = app.native_window().unwrap();
                    display = eglGetDisplay(ptr::null_mut());
                    eglInitialize(display, ptr::null_mut(), ptr::null_mut());

                    let attrs = [0x3033, 4, 0x3038];
                    let mut config = ptr::null_mut();
                    let mut num = 0;
                    eglChooseConfig(display, attrs.as_ptr(), &mut config, 1, &mut num);

                    surface = eglCreateWindowSurface(display, config, win.ptr().as_ptr() as *mut _, ptr::null());

                    // ES 3.0 контекст
                    let ctx_attrs = [0x3098, 3, 0x3038];
                    let context = eglCreateContext(display, config, ptr::null_mut(), ctx_attrs.as_ptr());

                    eglMakeCurrent(display, surface, surface, context);
                    eglSwapInterval(display, 1);

                    let gl_ctx = glow::Context::from_loader_function(|s| {
                        eglGetProcAddress(CString::new(s).unwrap().as_ptr())
                    });

                    gl = Some(gl_ctx);
                },
                PollEvent::Main(MainEvent::Destroy) => running = false,
                _ => {}
            }
        });

        if let Some(gl) = &gl {
            unsafe {
                gl.clear_color(0.05, 0.1, 0.2, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);

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
