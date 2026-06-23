use android_activity::{AndroidApp, MainEvent, PollEvent};
use glam::*;
use glow::*;
use log::info;
use std::ffi::c_void;
use std::ptr;

// ========== FFI BINDINGS (Заглушки для EGL и NDK) ==========
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
    fn ANativeWindow_setBuffersGeometry(window: *mut c_void, width: i32, height: i32, format: i32);
    fn eglGetProcAddress(procname: *const u8) -> *mut c_void;
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
enum GameState {
    Lobby,
    Game,
}

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

// ========== GL STRUCTURES ==========
struct GlContext {
    display: *mut c_void,
    surface: *mut c_void,
    context: *mut c_void,
    gl: Context,
}

struct RenderState {
    gl: Context,
    program: Program,
    u_color: UniformLocation,
    u_res: UniformLocation,
    vbo: Buffer,
}

unsafe fn init_egl(window: *mut c_void) -> Option<GlContext> {
    let display = eglGetDisplay(ptr::null_mut());
    eglInitialize(display, ptr::null_mut(), ptr::null_mut());

    let attribs = [EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT, EGL_NONE];
    let mut config: *mut c_void = ptr::null_mut();
    let mut num_configs: i32 = 0;
    eglChooseConfig(display, attribs.as_ptr(), &mut config, 1, &mut num_configs);

    let mut format: i32 = 0;
    eglGetConfigAttrib(display, config, EGL_NATIVE_VISUAL_ID, &mut format);
    ANativeWindow_setBuffersGeometry(window, 0, 0, format);

    let surface = eglCreateWindowSurface(display, config, window, ptr::null_mut());
    let ctx_attribs = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
    let context = eglCreateContext(display, config, ptr::null_mut(), ctx_attribs.as_ptr());

    eglMakeCurrent(display, surface, surface, context);

    let mut w = 0; let mut h = 0;
    eglQuerySurface(display, surface, EGL_WIDTH, &mut w);
    eglQuerySurface(display, surface, EGL_HEIGHT, &mut h);
    SCREEN_W = w as f32;
    SCREEN_H = h as f32;

    let gl = Context::from_loader_function(|sym| {
        eglGetProcAddress(sym.as_ptr() as *const u8) as *const c_void
    });

    Some(GlContext { display, surface, context, gl })
}

// ========== RENDERING ==========
unsafe fn draw_rect(rs: &RenderState, x: f32, y: f32, w: f32, h: f32, color: Vec4, sw: f32, sh: f32) {
    let gl = &rs.gl;
    let vertices: [f32; 12] = [x, y, x+w, y, x+w, y+h, x, y, x+w, y+h, x, y+h];

    gl.bind_buffer(ARRAY_BUFFER, Some(rs.vbo));
    gl.buffer_data_u8_slice(ARRAY_BUFFER, std::slice::from_raw_parts(
        vertices.as_ptr() as *const u8,
        std::mem::size_of_val(&vertices)
    ), DYNAMIC_DRAW);

    gl.uniform_4_f32(Some(&rs.u_color), color.x, color.y, color.z, color.w);
    gl.uniform_2_f32(Some(&rs.u_res), sw, sh);

    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_f32(0, 2, FLOAT, false, 0, 0);
    gl.draw_arrays(TRIANGLES, 0, 6);
    gl.disable_vertex_attrib_array(0);
}

unsafe fn draw_cubic_text(rs: &RenderState, text: &str, x: f32, y: f32, size: f32, color: Vec4, sw: f32, sh: f32) {
    let mut offset = 0.0;
    for c in text.chars() {
        if c != ' ' {
            draw_rect(rs, x + offset, y, size * 0.8, size, color, sw, sh);
        }
        offset += size * 1.2;
    }
}

unsafe fn draw_lobby(rs: &RenderState) {
    let w = SCREEN_W;
    let h = SCREEN_H;

    draw_rect(rs, 0.0, 0.0, w, h, vec4(0.55, 0.20, 0.85, 1.0), w, h);

    let title_size = 40.0;
    let title_w = "CUBIC BATTLE".len() as f32 * title_size * 1.2;
    draw_cubic_text(rs, "CUBIC BATTLE", w/2.0 - title_w/2.0, h/2.0 - 150.0, title_size, vec4(1.0, 1.0, 1.0, 1.0), w, h);

    let sub_size = 15.0;
    let sub_w = "TOUCH & DODGE".len() as f32 * sub_size * 1.2;
    draw_cubic_text(rs, "TOUCH & DODGE", w/2.0 - sub_w/2.0, h/2.0 - 60.0, sub_size, vec4(1.0, 1.0, 1.0, 1.0), w, h);

    let btn = LOBBY_BTN;
    draw_rect(rs, btn.x+5.0, btn.y+6.0, btn.w, btn.h, vec4(0.0, 0.0, 0.0, 0.2), w, h);
    draw_rect(rs, btn.x, btn.y, btn.w, btn.h, vec4(0.55, 0.20, 0.85, 1.0), w, h);

    let border = vec4(0.0, 0.0, 0.0, 1.0);
    let t = 3.4;
    draw_rect(rs, btn.x, btn.y, btn.w, t, border, w, h);
    draw_rect(rs, btn.x, btn.y + btn.h - t, btn.w, t, border, w, h);
    draw_rect(rs, btn.x, btn.y, t, btn.h, border, w, h);
    draw_rect(rs, btn.x + btn.w - t, btn.y, t, btn.h, border, w, h);

    let play_size = 20.0;
    let play_w = "PLAY".len() as f32 * play_size * 1.2;
    draw_cubic_text(rs, "PLAY", btn.x + btn.w/2.0 - play_w/2.0, btn.y + 20.0, play_size, vec4(1.0, 1.0, 1.0, 1.0), w, h);
}

// ========== MAIN ==========
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Trace));
    info!("Cubic Battle запущен на Rust!");

    let mut gl_ctx: Option<GlContext> = None;
    let mut render_state: Option<RenderState> = None;

    loop {
        app.poll_events(None, |event| {
            match event {
                PollEvent::Main(MainEvent::InitWindow { .. }) => {
                    let window = app.native_window().expect("No window");
                    unsafe {
                        if let Some(ctx) = init_egl(window.ptr().as_ptr() as *mut c_void) {
                            let gl = ctx.gl;
                            
                            place_lobby();

                            let vs = gl.create_shader(VERTEX_SHADER).unwrap();
                            gl.shader_source(vs, "attribute vec2 a_pos; uniform vec2 u_res; void main() { vec2 c = (a_pos / u_res) * 2.0 - 1.0; c.y *= -1.0; gl_Position = vec4(c, 0.0, 1.0); }");
                            gl.compile_shader(vs);

                            let fs = gl.create_shader(FRAGMENT_SHADER).unwrap();
                            gl.shader_source(fs, "precision mediump float; uniform vec4 u_color; void main() { gl_FragColor = u_color; }");
                            gl.compile_shader(fs);

                            let prog = gl.create_program().unwrap();
                            gl.attach_shader(prog, vs);
                            gl.attach_shader(prog, fs);
                            gl.link_program(prog);
                            gl.use_program(Some(prog));

                            let u_color = gl.get_uniform_location(prog, "u_color").unwrap();
                            let u_res = gl.get_uniform_location(prog, "u_res").unwrap();
                            let vbo = gl.create_buffer().unwrap();

                            render_state = Some(RenderState { gl, program: prog, u_color, u_res, vbo });
                            gl_ctx = Some(ctx);
                        }
                    }
                }
                PollEvent::Main(MainEvent::Destroy) => {
                    gl_ctx = None;
                    render_state = None;
                }
                PollEvent::Input(input_event) => {
                    if let Some(motion) = input_event.as_motion_event() {
                        if motion.get_action() == 0 { // ACTION_DOWN
                            let x = motion.get_x(0) as f32;
                            let y = motion.get_y(0) as f32;
                            unsafe {
                                if CURRENT_STATE == GameState::Lobby {
                                    let btn = LOBBY_BTN;
                                    if x >= btn.x && x <= btn.x + btn.w && y >= btn.y && y <= btn.y + btn.h {
                                        info!("Кнопка PLAY нажата!");
                                        CURRENT_STATE = GameState::Game;
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        });

        if let (Some(ctx), Some(rs)) = (&gl_ctx, &render_state) {
            let gl = &rs.gl;
            unsafe {
                gl.viewport(0, 0, SCREEN_W as i32, SCREEN_H as i32);
                gl.clear_color(0.1, 0.1, 0.15, 1.0);
                gl.clear(COLOR_BUFFER_BIT);
                
                match CURRENT_STATE {
                    GameState::Lobby => draw_lobby(rs),
                    GameState::Game => {
                        draw_rect(rs, 0.0, 0.0, SCREEN_W, SCREEN_H, vec4(0.2, 0.6, 1.0, 1.0), SCREEN_W, SCREEN_H);
                    }
                }

                eglSwapBuffers(ctx.display, ctx.surface);
            }
        }
    }
                }
