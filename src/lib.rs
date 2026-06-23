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
    fn eglQuerySurface(
        dpy: *mut c_void,
        surface: *mut c_void,
        attribute: i32,
        value: *mut i32,
    ) -> i32;
}

pub enum Scene {
    Lobby,
    Settings,
}

#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default());

    let mut scene = Scene::Lobby;
    let mut running = true;

    let mut display: *mut c_void = ptr::null_mut();
    let mut surface: *mut c_void = ptr::null_mut();
    let mut gl: Option<glow::Context> = None;

    let mut width = 0;
    let mut height = 0;

    while running {
        app.poll_events(None, |event| {
            match event {
                PollEvent::Main(MainEvent::InitWindow { .. }) => unsafe {
                    let window = app.native_window().unwrap();

                    display = eglGetDisplay(ptr::null_mut());
                    if display.is_null() { return; }

                    eglInitialize(display, ptr::null_mut(), ptr::null_mut());

                    let config_attribs = [
                        0x3024, 8,
                        0x3023, 8,
                        0x3022, 8,
                        0x3033, 4,
                        0x3038
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

                    let context_attribs = [0x3098, 3, 0x3038];

                    let context = eglCreateContext(
                        display,
                        config,
                        ptr::null_mut(),
                        context_attribs.as_ptr(),
                    );

                    eglMakeCurrent(display, surface, surface, context);
                    eglSwapInterval(display, 1);

                    eglQuerySurface(display, surface, 0x3057, &mut width);
                    eglQuerySurface(display, surface, 0x3056, &mut height);

                    let gl_ctx = glow::Context::from_loader_function(|s| {
                        eglGetProcAddress(CString::new(s).unwrap().as_ptr())
                    });

                    gl = Some(gl_ctx);
                }

                PollEvent::Main(MainEvent::Destroy) => running = false,
                _ => {}
            }
        });

        if let Some(gl) = &gl {
            unsafe {
                gl.viewport(0, 0, width, height);

                match scene {
                    Scene::Lobby => {
                        if lobby::render(gl, &app, width, height) {
                            scene = Scene::Settings;
                        }
                    }
                    Scene::Settings => {
                        if settings::render(gl, &app, width, height) {
                            scene = Scene::Lobby;
                        }
                    }
                }

                eglSwapBuffers(display, surface);
            }
        }
    }
}
