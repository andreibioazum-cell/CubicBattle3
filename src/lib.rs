mod lobby;
mod settings;

use android_activity::{AndroidApp, MainEvent, PollEvent};
use glow::HasContext;
use std::ffi::{c_void, CString};
use std::ptr;

#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(display_id: *mut c_void) -> *mut c_void;
    fn eglInitialize(dpy: *mut c_void, major: *mut i32, minor: *mut i32) -> i32;
    fn eglChooseConfig(dpy: *mut c_void, attr: *const i32, conf: *mut *mut c_void, size: i32, n: *mut i32) -> i32;
    fn eglCreateWindowSurface(dpy: *mut c_void, conf: *mut c_void, win: *mut c_void, attr: *const i32) -> *mut c_void;
    fn eglCreateContext(dpy: *mut c_void, conf: *mut c_void, share: *mut c_void, attr: *const i32) -> *mut c_void;
    fn eglMakeCurrent(dpy: *mut c_void, draw: *mut c_void, read: *mut c_void, ctx: *mut c_void) -> i32;
    fn eglSwapInterval(dpy: *mut c_void, interval: i32) -> i32;
    fn eglSwapBuffers(dpy: *mut c_void, surface: *mut c_void) -> i32;
    fn eglGetProcAddress(name: *const std::os::raw::c_char) -> *mut c_void;
    fn eglQuerySurface(dpy: *mut c_void, surf: *mut c_void, attr: i32, val: *mut i32) -> i32;
}

pub enum Scene { Lobby, Settings }

#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default());
    let mut scene = Scene::Lobby;
    let mut running = true;
    let (mut display, mut surface) = (ptr::null_mut(), ptr::null_mut());
    let mut gl: Option<glow::Context> = None;
    let (mut width, mut height) = (0, 0);

    while running {
        app.poll_events(None, |event| {
            if let PollEvent::Main(MainEvent::InitWindow { .. }) = event {
                unsafe {
                    let win = app.native_window().unwrap();
                    display = eglGetDisplay(ptr::null_mut());
                    eglInitialize(display, ptr::null_mut(), ptr::null_mut());
                    
                    let cfg_attr = [0x3024, 8, 0x3023, 8, 0x3022, 8, 0x3033, 4, 0x3038];
                    let mut config = ptr::null_mut();
                    let mut n = 0;
                    eglChooseConfig(display, cfg_attr.as_ptr(), &mut config, 1, &mut n);
                    
                    surface = eglCreateWindowSurface(display, config, win.ptr().as_ptr() as *mut _, ptr::null());
                    
                    let ctx_attr = [0x3098, 3, 0x3038];
                    let ctx = eglCreateContext(display, config, ptr::null_mut(), ctx_attr.as_ptr());
                    
                    eglMakeCurrent(display, surface, surface, ctx);
                    eglSwapInterval(display, 1);
                    eglQuerySurface(display, surface, 0x3057, &mut width);
                    eglQuerySurface(display, surface, 0x3056, &mut height);
                    
                    gl = Some(glow::Context::from_loader_function(|s| eglGetProcAddress(CString::new(s).unwrap().as_ptr())));
                }
            }
            if let PollEvent::Main(MainEvent::Destroy) = event { running = false; }
        });

        if let Some(gl_ctx) = &gl {
            unsafe {
                gl_ctx.viewport(0, 0, width, height);
                match scene {
                    Scene::Lobby => if lobby::render(gl_ctx, &app, width, height) { scene = Scene::Settings; },
                    Scene::Settings => if settings::render(gl_ctx, &app, width, height) { scene = Scene::Lobby; },
                }
                eglSwapBuffers(display, surface);
            }
        }
    }
}
