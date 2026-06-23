use android_activity::{AndroidApp, MainEvent, PollEvent};
use glam::{vec4, Vec4};
use glow::HasContext;
use log::info;
use std::ffi::{c_void, CString};
use std::ptr;

// ========== FFI BINDINGS (EGL & Android) ==========
#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(display_id: *mut c_void) -> *mut c_void;
    fn eglInitialize(display: *mut c_void, major: *mut i32, minor: *mut i32) -> i32;
    fn eglChooseConfig(display: *mut c_void, attrib_list: *const i32, configs: *mut *mut c_void, config_size: i32, num_config: *mut i32) -> i32;
    fn eglGetConfigAttrib(display: *mut c_void, config: *mut c_void, attribute: i32, value: *mut i32) -> i32;
    fn eglCreateWindowSurface(display: *mut c_void, config: *mut c_void, win: *mut c_void, attrib_list: *const i32) -> *mut c_void;
    fn eglCreateContext(display: *mut c_void, config: *mut c_void, share_context: *mut c_void, attrib_list: *const i32) -> *mut c_void;
    fn eglMakeCurrent(display: *mut c_void, draw: *mut c_void, read: *mut c_void, context: *mut c_void) -> i32;
    fn eglQuerySurface(display: *mut c_void, surface: *mut c_void, attribute: i32, value: *mut i32) -> i32;
    fn eglSwapBuffers(display: *mut c_void, surface: *mut c_void) -> i32;
    fn eglGetProcAddress(procname: *const std::os::raw::c_char) -> *mut c_void;
}

#[link(name = "android")]
extern "C" {
    fn ANativeWindow_setBuffersGeometry(window: *mut c_void, width: i32, height: i32, format: i32);
}

const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_OPENGL_ES2_BIT: i32 = 0x0004;
const EGL_NONE: i32 = 0x3038;
const EGL_NATIVE_VISUAL_ID: i32 = 0x302E;
const EGL_CONTEXT_CLIENT_VERSION: i32 = 0x3098;
const EGL_WIDTH: i32 = 0x3057;
const EGL_HEIGHT: i32 = 0x3056;

// ========== GAME STATE ==========
#[derive(Clone, Copy, PartialEq, Eq)]
enum GameState { Lobby, Game }

#[derive(Clone, Copy)]
struct Button { w: f32, h: f32, x: f32, y: f32 }

static mut CURRENT_STATE: GameState = GameState::Lobby;
static mut LOBBY_BTN: Button = Button { w: 220.0, h: 75.0, x: 0.0, y: 0.0 };
static mut SCREEN_W: f32 = 0.0;
static mut SCREEN_H: f32 = 0.0;

unsafe fn place_lobby() {
    LOBBY_BTN.x = SCREEN_W / 2.0 - LOBBY_BTN.w / 2.0;
    LOBBY_BTN.y = SCREEN_H / 2.0 + 50.0;
}

// ========== STRUCTURES ==========
struct GlContext {
    display: *mut c_void,
    surface: *mut c_void,
    gl: glow::Context,
}

struct RenderState {
    program: glow::Program,
    u_color: glow::UniformLocation,
    u_res: glow::UniformLocation,
    vbo: glow::Buffer,
}

unsafe fn init_egl(native_window: *mut c_void) -> Option<GlContext> {
    let display = eglGetDisplay(ptr::null_mut());
    eglInitialize(display, ptr::null_mut(), ptr::null_mut());

    let attribs = [EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT, EGL_NONE];
    let mut config: *mut c_void = ptr::null_mut();
    let mut num_configs: i32 = 0;
    eglChooseConfig(display, attribs.as_ptr(), &mut config, 1, &mut num_configs);

    let mut format: i32 = 0;
    eglGetConfigAttrib(display, config, EGL_NATIVE_VISUAL_ID, &mut format);
    ANativeWindow_setBuffersGeometry(native_window, 0, 0, format);

    let surface = eglCreateWindowSurface(display, config, native_window, ptr::null_mut());
    let ctx_attribs = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
    let context = eglCreateContext(display, config, ptr::null_mut(), ctx_attribs.as_ptr());

    eglMakeCurrent(display, surface, surface, context);

    let mut w = 0; let mut h = 0;
    eglQuerySurface(display, surface, EGL_WIDTH, &mut w);
    eglQuerySurface(display, surface, EGL_HEIGHT, &mut h);
    SCREEN_W = w as f32;
    SCREEN_H = h as f32;

    let gl = glow::Context::from_loader_function(|sym| {
        let c_str = CString::new(sym).unwrap();
        eglGetProcAddress(c_str.as_ptr() as *const _)
    });

    Some(GlContext { display, surface, gl })
}

unsafe fn init_renderer(gl: &glow::Context) -> RenderState {
    let vs_src = "attribute vec2 a_pos; uniform vec2 u_res; void main() { vec2 c = (a_pos / u_res) * 2.0 - 1.0; c.y *= -1.0; gl_Position = vec4(c, 0.0, 1.0); }";
    let fs_src = "precision mediump float; uniform vec4 u_color; void main() { gl_FragColor = u_color; }";

    let vs = gl.create_shader(glow::VERTEX_SHADER).unwrap();
    gl.shader_source(vs, vs_src);
    gl.compile_shader(vs);

    let fs = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
    gl.shader_source(fs, fs_src);
    gl.compile_shader(fs);

    let program = gl.create_program().unwrap();
    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);
    gl.bind_attrib_location(program, 0, "a_pos");
    gl.link_program(program);

    let u_color = gl.get_uniform_location(program, "u_color").unwrap();
    let u_res = gl.get_uniform_location(program, "u_res").unwrap();
    let vbo = gl.create_buffer().unwrap();

    gl.enable(glow::BLEND);
    gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

    RenderState { program, u_color, u_res, vbo }
}

unsafe fn draw_rect(gl: &glow::Context, rs: &RenderState, x: f32, y: f32, w: f32, h: f32, color: Vec4) {
    let vertices: [f32; 12] = [x, y, x + w, y, x + w, y + h, x, y, x + w, y + h, x, y + h];
    gl.use_program(Some(rs.program));
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(rs.vbo));
    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, std::slice::from_raw_parts(vertices.as_ptr() as *const u8, 48), glow::DYNAMIC_DRAW);
    gl.uniform_4_f32(Some(&rs.u_color), color.x, color.y, color.z, color.w);
    gl.uniform_2_f32(Some(&rs.u_res), SCREEN_W, SCREEN_H);
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
    gl.draw_arrays(glow::TRIANGLES, 0, 6);
}

unsafe fn draw_cubic_text(gl: &glow::Context, rs: &RenderState, text: &str, x: f32, y: f32, size: f32, color: Vec4) {
    let mut offset = 0.0;
    for _ in text.chars() {
        draw_rect(gl, rs, x + offset, y, size * 0.8, size, color);
        offset += size * 1.2;
    }
}

unsafe fn draw_lobby(gl: &glow::Context, rs: &RenderState) {
    draw_rect(gl, rs, 0.0, 0.0, SCREEN_W, SCREEN_H, vec4(0.55, 0.20, 0.85, 1.0));
    draw_cubic_text(gl, rs, "CUBIC BATTLE", SCREEN_W/2.0 - 200.0, SCREEN_H/2.0 - 150.0, 40.0, vec4(1.0, 1.0, 1.0, 1.0));
    
    let btn = LOBBY_BTN;
    draw_rect(gl, rs, btn.x+5.0, btn.y+6.0, btn.w, btn.h, vec4(0.0, 0.0, 0.0, 0.2));
    draw_rect(gl, rs, btn.x, btn.y, btn.w, btn.h, vec4(0.55, 0.20, 0.85, 1.0));
    
    let t = 3.4;
    let border = vec4(0.0, 0.0, 0.0, 1.0);
    draw_rect(gl, rs, btn.x, btn.y, btn.w, t, border);
    draw_rect(gl, rs, btn.x, btn.y + btn.h - t, btn.w, t, border);
    draw_rect(gl, rs, btn.x, btn.y, t, btn.h, border);
    draw_rect(gl, rs, btn.x + btn.w - t, btn.y, t, btn.h, border);
}

// ========== MAIN ==========
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Trace));

    let mut gl_ctx: Option<GlContext> = None;
    let mut rs: Option<RenderState> = None;

    loop {
        app.poll_events(Some(std::time::Duration::from_millis(0)), |event| match event {
            PollEvent::Main(MainEvent::InitWindow { .. }) => {
                let win = app.native_window().unwrap();
                unsafe {
                    if let Some(ctx) = init_egl(win.ptr().as_ptr() as *mut c_void) {
                        place_lobby();
                        rs = Some(init_renderer(&ctx.gl));
                        gl_ctx = Some(ctx);
                    }
                }
            }
            PollEvent::Main(MainEvent::Destroy) => {
                gl_ctx = None;
                rs = None;
            }
            _ => {}
        });

        // ПРАВИЛЬНЫЙ ВВОД для android-activity 0.5.2
        if let Ok(mut iter) = app.input_events_iter() {
            while iter.next(|input| {
                if let android_activity::input::InputEvent::MotionEvent(motion) = input {
                    if motion.action() == android_activity::input::MotionAction::Down {
                        let x = motion.pointer_at_index(0).x();
                        let y = motion.pointer_at_index(0).y();
                        unsafe {
                            if CURRENT_STATE == GameState::Lobby {
                                let btn = LOBBY_BTN;
                                if x >= btn.x && x <= btn.x + btn.w && y >= btn.y && y <= btn.y + btn.h {
                                    CURRENT_STATE = GameState::Game;
                                }
                            }
                        }
                    }
                }
            }) {}
        }

        if let (Some(ref ctx), Some(ref r)) = (&gl_ctx, &rs) {
            unsafe {
                ctx.gl.viewport(0, 0, SCREEN_W as i32, SCREEN_H as i32);
                ctx.gl.clear_color(0.1, 0.1, 0.15, 1.0);
                ctx.gl.clear(glow::COLOR_BUFFER_BIT);

                match CURRENT_STATE {
                    GameState::Lobby => draw_lobby(&ctx.gl, r),
                    GameState::Game => draw_rect(&ctx.gl, r, 0.0, 0.0, SCREEN_W, SCREEN_H, vec4(0.2, 0.6, 1.0, 1.0)),
                }
                eglSwapBuffers(ctx.display, ctx.surface);
            }
        }
    }
        }
