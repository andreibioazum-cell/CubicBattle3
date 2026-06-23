use android_activity::{AndroidApp, MainEvent, PollEvent};
use glam::{vec4, Vec4};
use glow::HasContext;
use log::info;
use std::ffi::{c_void, CString};
use std::ptr;

#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(display_id: *mut c_void) -> *mut c_void;
    fn eglInitialize(display: *mut c_void, major: *mut i32, minor: *mut i32) -> i32;
    fn eglChooseConfig(
        display: *mut c_void,
        attrib_list: *const i32,
        configs: *mut *mut c_void,
        config_size: i32,
        num_config: *mut i32,
    ) -> i32;
    fn eglGetConfigAttrib(display: *mut c_void, config: *mut c_void, attribute: i32, value: *mut i32)
        -> i32;
    fn eglCreateWindowSurface(
        display: *mut c_void,
        config: *mut c_void,
        win: *mut c_void,
        attrib_list: *const i32,
    ) -> *mut c_void;
    fn eglCreateContext(
        display: *mut c_void,
        config: *mut c_void,
        share_context: *mut c_void,
        attrib_list: *const i32,
    ) -> *mut c_void;
    fn eglMakeCurrent(display: *mut c_void, draw: *mut c_void, read: *mut c_void, context: *mut c_void)
        -> i32;
    fn eglQuerySurface(display: *mut c_void, surface: *mut c_void, attribute: i32, value: *mut i32)
        -> i32;
    fn eglSwapBuffers(display: *mut c_void, surface: *mut c_void) -> i32;
    fn eglGetProcAddress(procname: *const i8) -> *mut c_void;
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

// ====== GameState ======
#[derive(Clone, Copy, PartialEq, Eq)]
enum GameState {
    Lobby,
    Game,
}

#[derive(Clone, Copy)]
struct Button {
    w: f32,
    h: f32,
    x: f32,
    y: f32,
}

static mut CURRENT_STATE: GameState = GameState::Lobby;
static mut LOBBY_BTN: Button = Button {
    w: 220.0,
    h: 75.0,
    x: 0.0,
    y: 0.0,
};

static mut SCREEN_W: f32 = 0.0;
static mut SCREEN_H: f32 = 0.0;

unsafe fn place_lobby() {
    LOBBY_BTN.x = SCREEN_W / 2.0 - LOBBY_BTN.w / 2.0;
    LOBBY_BTN.y = SCREEN_H / 2.0 + 50.0;
}

// ====== EGL/GL context ======
struct GlContext {
    display: *mut c_void,
    surface: *mut c_void,
    _context: *mut c_void,
    gl: glow::Context,
}

struct RenderState {
    gl: glow::Context,
    program: glow::Program,
    u_color: glow::UniformLocation,
    u_res: glow::UniformLocation,
    vbo: glow::Buffer,
}

unsafe fn init_egl(native_window: *mut c_void) -> Option<GlContext> {
    let display = eglGetDisplay(ptr::null_mut());
    if display.is_null() {
        return None;
    }
    eglInitialize(display, ptr::null_mut(), ptr::null_mut());

    let attribs = [EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT, EGL_NONE];
    let mut config: *mut c_void = ptr::null_mut();
    let mut num_configs: i32 = 0;
    eglChooseConfig(display, attribs.as_ptr(), &mut config, 1, &mut num_configs);
    if num_configs <= 0 || config.is_null() {
        return None;
    }

    let mut format: i32 = 0;
    eglGetConfigAttrib(display, config, EGL_NATIVE_VISUAL_ID, &mut format);
    ANativeWindow_setBuffersGeometry(native_window, 0, 0, format);

    let surface = eglCreateWindowSurface(display, config, native_window, ptr::null_mut());
    if surface.is_null() {
        return None;
    }

    let ctx_attribs = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
    let context = eglCreateContext(display, config, ptr::null_mut(), ctx_attribs.as_ptr());
    if context.is_null() {
        return None;
    }

    eglMakeCurrent(display, surface, surface, context);

    let mut w = 0;
    let mut h = 0;
    eglQuerySurface(display, surface, EGL_WIDTH, &mut w);
    eglQuerySurface(display, surface, EGL_HEIGHT, &mut h);

    SCREEN_W = w as f32;
    SCREEN_H = h as f32;

    let gl = glow::Context::from_loader_function(|sym| {
        let c = CString::new(sym).unwrap();
        eglGetProcAddress(c.as_ptr()) as *const c_void
    });

    Some(GlContext {
        display,
        surface,
        _context: context,
        gl,
    })
}

unsafe fn compile_shader(gl: &glow::Context, ty: u32, src: &str) -> Result<glow::Shader, String> {
    let sh = gl.create_shader(ty).map_err(|e| e.to_string())?;
    gl.shader_source(sh, src);
    gl.compile_shader(sh);
    if !gl.get_shader_compile_status(sh) {
        let log = gl.get_shader_info_log(sh);
        gl.delete_shader(sh);
        return Err(log);
    }
    Ok(sh)
}

unsafe fn init_renderer(gl: &glow::Context) -> Result<RenderState, String> {
    let vs_src = r#"
        attribute vec2 a_pos;
        uniform vec2 u_res;
        void main() {
            vec2 c = (a_pos / u_res) * 2.0 - 1.0;
            c.y *= -1.0;
            gl_Position = vec4(c, 0.0, 1.0);
        }
    "#;

    let fs_src = r#"
        precision mediump float;
        uniform vec4 u_color;
        void main() {
            gl_FragColor = u_color;
        }
    "#;

    let vs = compile_shader(gl, glow::VERTEX_SHADER, vs_src)?;
    let fs = compile_shader(gl, glow::FRAGMENT_SHADER, fs_src)?;

    let program = gl.create_program().map_err(|e| e.to_string())?;
    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);

    // Важно: фиксируем a_pos в location=0 (иначе не гарантировано)
    gl.bind_attrib_location(program, 0, "a_pos");

    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        let log = gl.get_program_info_log(program);
        gl.delete_program(program);
        return Err(log);
    }

    gl.delete_shader(vs);
    gl.delete_shader(fs);

    gl.use_program(Some(program));

    let u_color = gl
        .get_uniform_location(program, "u_color")
        .ok_or("no uniform u_color")?;
    let u_res = gl.get_uniform_location(program, "u_res").ok_or("no uniform u_res")?;

    let vbo = gl.create_buffer().map_err(|e| e.to_string())?;

    gl.enable(glow::BLEND);
    gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

    Ok(RenderState {
        gl: gl.clone(),
        program,
        u_color,
        u_res,
        vbo,
    })
}

unsafe fn draw_rect(rs: &RenderState, x: f32, y: f32, w: f32, h: f32, color: Vec4) {
    let gl = &rs.gl;

    let vertices: [f32; 12] = [x, y, x + w, y, x + w, y + h, x, y, x + w, y + h, x, y + h];

    gl.use_program(Some(rs.program));
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(rs.vbo));
    gl.buffer_data_u8_slice(
        glow::ARRAY_BUFFER,
        std::slice::from_raw_parts(vertices.as_ptr() as *const u8, std::mem::size_of_val(&vertices)),
        glow::DYNAMIC_DRAW,
    );

    gl.uniform_4_f32(Some(&rs.u_color), color.x, color.y, color.z, color.w);
    gl.uniform_2_f32(Some(&rs.u_res), SCREEN_W, SCREEN_H);

    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
    gl.draw_arrays(glow::TRIANGLES, 0, 6);
    gl.disable_vertex_attrib_array(0);
}

// Простой “кубический” текст: каждая буква = прямоугольник
unsafe fn draw_cubic_text(rs: &RenderState, text: &str, x: f32, y: f32, size: f32, color: Vec4) {
    let mut offset = 0.0;
    for ch in text.chars() {
        if ch != ' ' {
            draw_rect(rs, x + offset, y, size * 0.8, size, color);
        }
        offset += size * 1.2;
    }
}

unsafe fn draw_lobby(rs: &RenderState) {
    let w = SCREEN_W;
    let h = SCREEN_H;

    // фон (пока однотонный, потом можно сделать градиент отдельным шейдером)
    draw_rect(rs, 0.0, 0.0, w, h, vec4(0.55, 0.20, 0.85, 1.0));

    // Заголовок
    let title_size = 40.0;
    let title = "CUBIC BATTLE";
    let title_w = title.len() as f32 * title_size * 1.2;
    draw_cubic_text(
        rs,
        title,
        w / 2.0 - title_w / 2.0,
        h / 2.0 - 150.0,
        title_size,
        vec4(1.0, 1.0, 1.0, 1.0),
    );

    // Подзаголовок
    let sub_size = 15.0;
    let sub = "TOUCH & DODGE";
    let sub_w = sub.len() as f32 * sub_size * 1.2;
    draw_cubic_text(
        rs,
        sub,
        w / 2.0 - sub_w / 2.0,
        h / 2.0 - 60.0,
        sub_size,
        vec4(1.0, 1.0, 1.0, 1.0),
    );

    // Кнопка
    let btn = LOBBY_BTN;

    // тень
    draw_rect(rs, btn.x + 5.0, btn.y + 6.0, btn.w, btn.h, vec4(0.0, 0.0, 0.0, 0.20));

    // тело кнопки
    draw_rect(rs, btn.x, btn.y, btn.w, btn.h, vec4(0.55, 0.20, 0.85, 1.0));

    // обводка (4 прямоугольника)
    let t = 3.4;
    let border = vec4(0.0, 0.0, 0.0, 1.0);
    draw_rect(rs, btn.x, btn.y, btn.w, t, border);
    draw_rect(rs, btn.x, btn.y + btn.h - t, btn.w, t, border);
    draw_rect(rs, btn.x, btn.y, t, btn.h, border);
    draw_rect(rs, btn.x + btn.w - t, btn.y, t, btn.h, border);

    // надпись PLAY
    let play_size = 20.0;
    let play = "PLAY";
    let play_w = play.len() as f32 * play_size * 1.2;
    draw_cubic_text(
        rs,
        play,
        btn.x + btn.w / 2.0 - play_w / 2.0,
        btn.y + 20.0,
        play_size,
        vec4(1.0, 1.0, 1.0, 1.0),
    );
}

#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Trace));
    info!("CubicBattle (Rust) start");

    let mut gl_ctx: Option<GlContext> = None;
    let mut rs: Option<RenderState> = None;

    loop {
        // 1) события Android (создание/уничтожение окна)
        app.poll_events(None, |event| match event {
            PollEvent::Main(MainEvent::InitWindow { .. }) => {
                let win = app.native_window().expect("No window");
                unsafe {
                    gl_ctx = init_egl(win.ptr().as_ptr() as *mut c_void);
                    if let Some(ref ctx) = gl_ctx {
                        place_lobby();
                        match init_renderer(&ctx.gl) {
                            Ok(r) => rs = Some(r),
                            Err(e) => {
                                rs = None;
                                info!("GL init error: {e}");
                            }
                        }
                    }
                }
            }
            PollEvent::Main(MainEvent::Destroy) => {
                gl_ctx = None;
                rs = None;
            }
            _ => {}
        });

        // 2) ввод (android-activity 0.5.2)
        unsafe {
            for input in app.input_events_iter() {
                if let Some(motion) = input.as_motion_event() {
                    let action = motion.get_action() & 0xff; // mask
                    if action == 0 {
                        let x = motion.get_x(0) as f32;
                        let y = motion.get_y(0) as f32;

                        if CURRENT_STATE == GameState::Lobby {
                            let btn = LOBBY_BTN;
                            if x >= btn.x && x <= btn.x + btn.w && y >= btn.y && y <= btn.y + btn.h {
                                CURRENT_STATE = GameState::Game;
                            }
                        }
                    }
                }
            }
        }

        // 3) рендер
        if let (Some(ref ctx), Some(ref rs)) = (&gl_ctx, &rs) {
            unsafe {
                rs.gl.viewport(0, 0, SCREEN_W as i32, SCREEN_H as i32);
                rs.gl.clear_color(0.1, 0.1, 0.15, 1.0);
                rs.gl.clear(glow::COLOR_BUFFER_BIT);

                match unsafe { CURRENT_STATE } {
                    GameState::Lobby => unsafe { draw_lobby(rs) },
                    GameState::Game => unsafe {
                        draw_rect(rs, 0.0, 0.0, SCREEN_W, SCREEN_H, vec4(0.2, 0.6, 1.0, 1.0))
                    },
                }

                eglSwapBuffers(ctx.display, ctx.surface);
            }
        }
    }
        }
