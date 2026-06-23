use android_activity::{AndroidApp, MainEvent, PollEvent, InputStatus};
use glam::{vec4, Vec4};
use glow::HasContext;
use std::ffi::{c_void, CString};
use std::ptr;

// ========== EGL BINDINGS ==========
#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(id: *mut c_void) -> *mut c_void;
    fn eglInitialize(d: *mut c_void, ma: *mut i32, mi: *mut i32) -> i32;
    fn eglChooseConfig(d: *mut c_void, al: *const i32, c: *mut *mut c_void, cs: i32, nc: *mut i32) -> i32;
    fn eglGetConfigAttrib(d: *mut c_void, c: *mut c_void, a: i32, v: *mut i32) -> i32;
    fn eglCreateWindowSurface(d: *mut c_void, c: *mut c_void, w: *mut c_void, al: *const i32) -> *mut c_void;
    fn eglCreateContext(d: *mut c_void, c: *mut c_void, sc: *mut c_void, al: *const i32) -> *mut c_void;
    fn eglMakeCurrent(d: *mut c_void, dr: *mut c_void, r: *mut c_void, ctx: *mut c_void) -> i32;
    fn eglQuerySurface(d: *mut c_void, s: *mut c_void, a: i32, v: *mut i32) -> i32;
    fn eglSwapBuffers(d: *mut c_void, s: *mut c_void) -> i32;
    fn eglGetProcAddress(n: *const std::os::raw::c_char) -> *mut c_void;
}
#[link(name = "android")]
extern "C" { fn ANativeWindow_setBuffersGeometry(w: *mut c_void, wi: i32, h: i32, f: i32); }

const EGL_OPENGL_ES2_BIT: i32 = 4;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_NONE: i32 = 0x3038;
const EGL_NATIVE_VISUAL_ID: i32 = 0x302E;
const EGL_CONTEXT_CLIENT_VERSION: i32 = 0x3098;
const EGL_WIDTH: i32 = 0x3057;
const EGL_HEIGHT: i32 = 0x3056;

// ========== ПРОСТОЙ ПИКСЕЛЬНЫЙ ШРИФТ (5x5) ==========
fn get_char_pixels(c: char) -> &'static [u8] {
    match c.to_ascii_uppercase() {
        'C' => &[0,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 0,1,1,1,0],
        'U' => &[1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
        'B' => &[1,1,1,1,0, 1,0,0,0,1, 1,1,1,1,0, 1,0,0,0,1, 1,1,1,1,0],
        'I' => &[0,1,1,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,1,1,0],
        'A' => &[0,1,1,1,0, 1,0,0,0,1, 1,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1],
        'T' => &[1,1,1,1,1, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0],
        'L' => &[1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,1],
        'E' => &[1,1,1,1,1, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,0, 1,1,1,1,1],
        'O' => &[0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
        'D' => &[1,1,1,0,0, 1,0,0,1,0, 1,0,0,1,0, 1,0,0,1,0, 1,1,1,0,0],
        'G' => &[0,1,1,1,0, 1,0,0,0,0, 1,0,1,1,1, 1,0,0,0,1, 0,1,1,1,0],
        'H' => &[1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1],
        'P' => &[1,1,1,1,0, 1,0,0,0,1, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0],
        'Y' => &[1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0, 0,0,1,0,0, 0,0,1,0,0],
        '&' => &[0,1,1,0,0, 1,0,0,1,0, 0,1,1,0,0, 1,0,0,1,1, 0,1,1,0,1],
        _ => &[0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
    }
}

// ========== GAME STATE ==========
#[derive(Clone, Copy, PartialEq)]
enum GameState { Lobby, Game }
static mut STATE: GameState = GameState::Lobby;
static mut SW: f32 = 0.0;
static mut SH: f32 = 0.0;

#[derive(Clone, Copy)]
struct Button { x: f32, y: f32, w: f32, h: f32 }
static mut BTN: Button = Button { x: 0.0, y: 0.0, w: 220.0, h: 75.0 };

// ========== SHADERS ==========
const VS: &str = r#"
    attribute vec2 a_pos;
    varying vec2 v_uv;
    void main() {
        v_uv = a_pos;
        gl_Position = vec4(a_pos * 2.0 - 1.0, 0.0, 1.0);
    }
"#;

const FS: &str = r#"
    precision mediump float;
    varying vec2 v_uv;
    uniform int u_mode;
    uniform vec2 u_res;
    uniform vec4 u_c1, u_c2, u_c3, u_c4;
    uniform vec4 u_rect;
    uniform float u_radius;
    uniform vec4 u_color;

    float sdf_rect(vec2 p, vec2 b, float r) {
        vec2 d = abs(p - b * 0.5) - b * 0.5 + r;
        return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0) - r;
    }

    void main() {
        if (u_mode == 0) {
            vec4 top = mix(u_c1, u_c2, v_uv.x);
            vec4 bot = mix(u_c4, u_c3, v_uv.x);
            gl_FragColor = mix(bot, top, v_uv.y);
        } else {
            vec2 px = vec2(v_uv.x, 1.0 - v_uv.y) * u_res;
            float d = sdf_rect(px - u_rect.xy, u_rect.zw, u_radius);
            if (d > 0.0) discard;
            gl_FragColor = u_color;
        }
    }
"#;

struct Render { gl: glow::Context, prog: glow::Program }

unsafe fn draw_rect(r: &Render, x: f32, y: f32, w: f32, h: f32, rad: f32, color: Vec4) {
    let gl = &r.gl;
    gl.uniform_1_i32(gl.get_uniform_location(r.prog, "u_mode").as_ref(), 1);
    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_rect").as_ref(), x, y, w, h);
    gl.uniform_1_f32(gl.get_uniform_location(r.prog, "u_radius").as_ref(), rad);
    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_color").as_ref(), color.x, color.y, color.z, color.w);
    gl.draw_arrays(glow::TRIANGLES, 0, 6);
}

unsafe fn draw_text(r: &Render, text: &str, mut x: f32, y: f32, scale: f32, color: Vec4) {
    for c in text.chars() {
        if c == ' ' { x += scale * 6.0; continue; }
        let pixels = get_char_pixels(c);
        for row in 0..5 {
            for col in 0..5 {
                if pixels[row * 5 + col] == 1 {
                    draw_rect(r, x + col as f32 * scale, y + row as f32 * scale, scale, scale, 0.0, color);
                }
            }
        }
        x += scale * 6.0;
    }
}

// ========== MAIN ==========
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    let mut gl_ctx: Option<(*mut c_void, *mut c_void)> = None;
    let mut ren: Option<Render> = None;

    loop {
        app.poll_events(Some(std::time::Duration::from_millis(1)), |event| {
            if let PollEvent::Main(MainEvent::InitWindow { .. }) = event {
                let win = app.native_window().unwrap();
                unsafe {
                    let d = eglGetDisplay(ptr::null_mut());
                    eglInitialize(d, ptr::null_mut(), ptr::null_mut());
                    let attr = [EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT, EGL_NONE];
                    let mut cfg: *mut c_void = ptr::null_mut();
                    let mut n = 0;
                    eglChooseConfig(d, attr.as_ptr(), &mut cfg, 1, &mut n);
                    let mut f = 0;
                    eglGetConfigAttrib(d, cfg, EGL_NATIVE_VISUAL_ID, &mut f);
                    ANativeWindow_setBuffersGeometry(win.ptr().as_ptr() as *mut _, 0, 0, f);
                    let s = eglCreateWindowSurface(d, cfg, win.ptr().as_ptr() as *mut _, ptr::null_mut());
                    let ctx_attr = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
                    let c = eglCreateContext(d, cfg, ptr::null_mut(), ctx_attr.as_ptr());
                    eglMakeCurrent(d, s, s, c);
                    let mut width = 0; let mut height = 0;
                    eglQuerySurface(d, s, EGL_WIDTH, &mut width);
                    eglQuerySurface(d, s, EGL_HEIGHT, &mut height);
                    SW = width as f32; SH = height as f32;
                    BTN.x = SW/2.0 - BTN.w/2.0; BTN.y = SH/2.0 + 50.0;
                    let gl = glow::Context::from_loader_function(|s| {
                        let n = CString::new(s).unwrap();
                        eglGetProcAddress(n.as_ptr())
                    });
                    let p = gl.create_program().unwrap();
                    let vs = gl.create_shader(glow::VERTEX_SHADER).unwrap();
                    gl.shader_source(vs, VS); gl.compile_shader(vs);
                    let fs = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
                    gl.shader_source(fs, FS); gl.compile_shader(fs);
                    gl.attach_shader(p, vs); gl.attach_shader(p, fs);
                    gl.link_program(p); gl.use_program(Some(p));
                    let vbo = gl.create_buffer().unwrap();
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                    let verts: [f32; 12] = [0.0,0.0, 1.0,0.0, 1.0,1.0, 0.0,0.0, 1.0,1.0, 0.0,1.0];
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, std::slice::from_raw_parts(verts.as_ptr() as *const u8, 48), glow::STATIC_DRAW);
                    gl.enable_vertex_attrib_array(0);
                    gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
                    gl.enable(glow::BLEND);
                    gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                    gl_ctx = Some((d, s)); ren = Some(Render { gl, prog: p });
                }
            }
        });

        if let Ok(mut iter) = app.input_events_iter() {
            while iter.next(|ev| {
                if let android_activity::input::InputEvent::MotionEvent(m) = ev {
                    if m.action() == android_activity::input::MotionAction::Down {
                        let x = m.pointer_at_index(0).x(); let y = m.pointer_at_index(0).y();
                        unsafe { if STATE == GameState::Lobby && x > BTN.x && x < BTN.x+BTN.w && y > BTN.y && y < BTN.y+BTN.h { STATE = GameState::Game; } }
                    }
                }
                InputStatus::Handled
            }) {}
        }

        if let (Some((d, s)), Some(r)) = (gl_ctx, &ren) {
            let gl = &r.gl;
            unsafe {
                gl.viewport(0, 0, SW as i32, SH as i32);
                gl.use_program(Some(r.prog));
                gl.uniform_2_f32(gl.get_uniform_location(r.prog, "u_res").as_ref(), SW, SH);

                if STATE == GameState::Lobby {
                    gl.uniform_1_i32(gl.get_uniform_location(r.prog, "u_mode").as_ref(), 0);
                    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_c1").as_ref(), 0.45, 0.15, 0.80, 1.0);
                    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_c2").as_ref(), 0.55, 0.20, 0.85, 1.0);
                    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_c3").as_ref(), 0.85, 0.30, 0.65, 1.0);
                    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_c4").as_ref(), 0.80, 0.25, 0.70, 1.0);
                    gl.draw_arrays(glow::TRIANGLES, 0, 6);

                    draw_text(r, "CUBIC BATTLE", SW/2.0 - 190.0, SH/2.0 - 150.0, 6.0, vec4(1.0, 1.0, 1.0, 1.0));
                    draw_text(r, "TOUCH & DODGE", SW/2.0 - 100.0, SH/2.0 - 60.0, 2.0, vec4(1.0, 1.0, 1.0, 1.0));

                    let b = BTN;
                    draw_rect(r, b.x + 5.0, b.y + 6.0, b.w, b.h, 16.0, vec4(0.0, 0.0, 0.0, 0.2));
                    draw_rect(r, b.x - 2.0, b.y - 2.0, b.w + 4.0, b.h + 4.0, 18.0, vec4(0.0, 0.0, 0.0, 1.0));
                    draw_rect(r, b.x, b.y, b.w, b.h, 16.0, vec4(0.55, 0.20, 0.85, 1.0));
                    draw_text(r, "PLAY", b.x + 85.0, b.y + 25.0, 3.0, vec4(1.0, 1.0, 1.0, 1.0));
                } else {
                    gl.clear_color(0.2, 0.6, 1.0, 1.0); gl.clear(glow::COLOR_BUFFER_BIT);
                }
                eglSwapBuffers(d, s);
            }
        }
    }
         }
