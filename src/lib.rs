 use android_activity::{AndroidApp, MainEvent, PollEvent, InputStatus};
use glam::{vec4, Vec4, Vec2};
use glow::HasContext;
use log::info;
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

// ========== GAME LOGIC & OBJECTS ==========
#[derive(Clone, Copy, PartialEq)]
enum GameState { Lobby, Game }
static mut STATE: GameState = GameState::Lobby;

struct Star { x: f32, y: f32, speed: f32, alpha: f32, size: f32 }
static mut STARS: Vec<Star> = Vec::new();
static mut SW: f32 = 0.0;
static mut SH: f32 = 0.0;

#[derive(Clone, Copy)]
struct Button { x: f32, y: f32, w: f32, h: f32 }
static mut BTN_PLAY: Button = Button { x: 0.0, y: 0.0, w: 220.0, h: 75.0 };

unsafe fn get_scale() -> f32 { (if SW < SH { SW } else { SH }) / 600.0 }

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
    uniform int u_mode; // 0: Grad, 1: SDF Rect, 2: SDF Star
    uniform vec2 u_res;
    uniform vec4 u_c1, u_c2, u_c3, u_c4; // Corners
    uniform vec4 u_rect; // x, y, w, h
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
        } else if (u_mode == 1) {
            vec2 px = vec2(v_uv.x, 1.0 - v_uv.y) * u_res;
            float d = sdf_rect(px - u_rect.xy, u_rect.zw, u_radius);
            float alpha = smoothstep(1.0, 0.0, d);
            if (alpha <= 0.0) discard;
            gl_FragColor = vec4(u_color.rgb, u_color.a * alpha);
        } else {
            gl_FragColor = u_color; // Simple for stars
        }
    }
"#;

struct Render { gl: glow::Context, prog: glow::Program }

unsafe fn draw_rounded_rect(r: &Render, x: f32, y: f32, w: f32, h: f32, rad: f32, color: Vec4) {
    let gl = &r.gl;
    gl.uniform_1_i32(gl.get_uniform_location(r.prog, "u_mode").as_ref(), 1);
    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_rect").as_ref(), x, y, w, h);
    gl.uniform_1_f32(gl.get_uniform_location(r.prog, "u_radius").as_ref(), rad);
    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_color").as_ref(), color.x, color.y, color.z, color.w);
    gl.draw_arrays(glow::TRIANGLES, 0, 6);
}

// ========== MAIN ENTRY ==========
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default());
    let mut gl_ctx: Option<(*mut c_void, *mut c_void)> = None;
    let mut ren: Option<Render> = None;

    loop {
        app.poll_events(Some(std::time::Duration::from_millis(16)), |event| {
            if let PollEvent::Main(MainEvent::InitWindow { .. }) = event {
                let win = app.native_window().unwrap();
                unsafe {
                    let d = eglGetDisplay(ptr::null_mut());
                    eglInitialize(d, ptr::null_mut(), ptr::null_mut());
                    let mut cfg: *mut c_void = ptr::null_mut();
                    let mut n = 0;
                    eglChooseConfig(d, [0x3040, 4, 0x3038].as_ptr(), &mut cfg, 1, &mut n);
                    let mut f = 0;
                    eglGetConfigAttrib(d, cfg, 0x302E, &mut f);
                    ANativeWindow_setBuffersGeometry(win.ptr().as_ptr() as *mut _, 0, 0, f);
                    let s = eglCreateWindowSurface(d, cfg, win.ptr().as_ptr() as *mut _, ptr::null_mut());
                    let c = eglCreateContext(d, cfg, ptr::null_mut(), [0x3098, 2, 0x3038].as_ptr());
                    eglMakeCurrent(d, s, s, c);
                    
                    let mut width = 0; let mut height = 0;
                    eglQuerySurface(d, s, 0x3057, &mut width); eglQuerySurface(d, s, 0x3056, &mut height);
                    SW = width as f32; SH = height as f32;
                    let sc = get_scale();
                    BTN_PLAY.x = SW/2.0 - (BTN_PLAY.w * sc)/2.0; BTN_PLAY.y = SH/2.0 + 80.0 * sc;

                    STARS = (0..100).map(|_| Star {
                        x: rand::random::<f32>() * SW,
                        y: rand::random::<f32>() * SH,
                        speed: 40.0 + rand::random::<f32>() * 80.0,
                        alpha: 0.5 + rand::random::<f32>() * 0.5,
                        size: 1.0 + rand::random::<f32>() * 2.0,
                    }).collect();

                    let gl = glow::Context::from_loader_function(|s| eglGetProcAddress(CString::new(s).unwrap().as_ptr()));
                    let p = gl.create_program().unwrap();
                    let vs = gl.create_shader(glow::VERTEX_SHADER).unwrap(); gl.shader_source(vs, VS); gl.compile_shader(vs);
                    let fs = gl.create_shader(glow::FRAGMENT_SHADER).unwrap(); gl.shader_source(fs, FS); gl.compile_shader(fs);
                    gl.attach_shader(p, vs); gl.attach_shader(p, fs); gl.link_program(p); gl.use_program(Some(p));
                    let vbo = gl.create_buffer().unwrap(); gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                    let verts: [f32; 12] = [0.0,0.0, 1.0,0.0, 1.0,1.0, 0.0,0.0, 1.0,1.0, 0.0,1.0];
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, std::slice::from_raw_parts(verts.as_ptr() as *const u8, 48), glow::STATIC_DRAW);
                    gl.enable_vertex_attrib_array(0); gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
                    gl.enable(glow::BLEND); gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                    
                    gl_ctx = Some((d, s)); ren = Some(Render { gl, prog: p });
                }
            }
        });

        // Update Stars
        unsafe {
            let dt = 0.016;
            for s in &mut STARS {
                s.x += s.speed * dt; s.y += s.speed * dt;
                if s.x > SW || s.y > SH { s.x = 0.0; s.y = rand::random::<f32>() * SH; }
            }
        }

        if let Ok(mut iter) = app.input_events_iter() {
            while iter.next(|ev| {
                if let android_activity::input::InputEvent::MotionEvent(m) = ev {
                    if m.action() == android_activity::input::MotionAction::Down {
                        let x = m.pointer_at_index(0).x(); let y = m.pointer_at_index(0).y();
                        unsafe {
                            let sc = get_scale();
                            if STATE == GameState::Lobby && x > BTN_PLAY.x && x < BTN_PLAY.x + BTN_PLAY.w * sc && y > BTN_PLAY.y && y < BTN_PLAY.y + BTN_PLAY.h * sc {
                                STATE = GameState::Game;
                            }
                        }
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
                    // 1. Градиент
                    gl.uniform_1_i32(gl.get_uniform_location(r.prog, "u_mode").as_ref(), 0);
                    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_c1").as_ref(), 0.02, 0.00, 0.10, 1.0);
                    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_c2").as_ref(), 0.00, 0.05, 0.25, 1.0);
                    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_c3").as_ref(), 0.10, 0.15, 0.45, 1.0);
                    gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_c4").as_ref(), 0.05, 0.10, 0.35, 1.0);
                    gl.draw_arrays(glow::TRIANGLES, 0, 6);

                    // 2. Звезды
                    gl.uniform_1_i32(gl.get_uniform_location(r.prog, "u_mode").as_ref(), 2);
                    for star in &STARS {
                        gl.uniform_4_f32(gl.get_uniform_location(r.prog, "u_color").as_ref(), 1.0, 1.0, 1.0, star.alpha);
                        draw_rounded_rect(gl, r.prog, star.x, star.y, star.size, star.size, 0.0, vec4(1.0,1.0,1.0, star.alpha));
                    }

                    // 3. Кнопка Play (пока без текста, но с формой и тенью)
                    let sc = get_scale();
                    let b = BTN_PLAY;
                    draw_rounded_rect(gl, r.prog, b.x + 5.0*sc, b.y + 6.0*sc, b.w*sc, b.h*sc, 16.0*sc, vec4(0.1, 0.0, 0.2, 0.5));
                    draw_rounded_rect(gl, r.prog, b.x, b.y, b.w*sc, b.h*sc, 16.0*sc, vec4(0.35, 0.15, 0.75, 1.0));
                } else {
                    gl.clear_color(0.2, 0.6, 1.0, 1.0); gl.clear(glow::COLOR_BUFFER_BIT);
                }
                eglSwapBuffers(d, s);
            }
        }
    }
}
