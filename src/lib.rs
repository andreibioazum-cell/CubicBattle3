use android_activity::{AndroidApp, MainEvent, PollEvent};
use glam::*;
use glow::*;
use libc::*;
use log::info;
use std::ffi::c_void;
use std::ptr;

// ========== GAME STATE (из Lua) ==========
enum GameState {
    Lobby,
    Game,
}
static mut CURRENT_STATE: GameState = GameState::Lobby;

// ========== LOBBY UI (из Lua) ==========
struct Button {
    w: f32,
    h: f32,
    x: f32,
    y: f32,
}

static mut LOBBY_BTN: Button = Button { w: 220.0, h: 75.0, x: 0.0, y: 0.0 };
static mut SCREEN_W: f32 = 0.0;
static mut SCREEN_H: f32 = 0.0;

unsafe fn place_lobby() {
    LOBBY_BTN.x = SCREEN_W / 2.0 - LOBBY_BTN.w / 2.0;
    LOBBY_BTN.y = SCREEN_H / 2.0 + 50.0;
}

// ========== EGL & OPENGL SETUP ==========
struct GlContext {
    display: *mut c_void,
    surface: *mut c_void,
    context: *mut c_void,
    gl: Context,
}

unsafe fn init_egl(window: *mut ANativeWindow) -> Option<GlContext> {
    let display = eglGetDisplay(ptr::null_mut());
    eglInitialize(display, ptr::null_mut(), ptr::null_mut());

    let attribs = [
        EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT,
        EGL_NONE,
    ];
    let mut config: *mut c_void = ptr::null_mut();
    let mut num_configs: i32 = 0;
    eglChooseConfig(display, attribs.as_ptr(), &mut config, 1, &mut num_configs);

    let format: i32 = 0;
    eglGetConfigAttrib(display, config, EGL_NATIVE_VISUAL_ID, &format as *const i32 as *mut i32);
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
        eglGetProcAddress(sym.as_ptr() as *const i8) as *const c_void
    });

    Some(GlContext { display, surface, context, gl })
}

// ========== RENDERING ==========
unsafe fn draw_rect(gl: &Context, x: f32, y: f32, w: f32, h: f32, color: Vec4, sw: f32, sh: f32) {
    let vertices: [f32; 12] = [x, y, x+w, y, x+w, y+h, x, y, x+w, y+h, x, y+h];
    gl.uniform_4_f32(1, color.x, color.y, color.z, color.w);
    gl.uniform_2_f32(2, sw, sh);
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_f32(0, 2, FALSE, 0, vertices.as_ptr() as *const c_void);
    gl.draw_arrays(TRIANGLES, 0, 6);
    gl.disable_vertex_attrib_array(0);
}

// Рисуем буквы из кубиков (Cubic стиль!)
unsafe fn draw_cubic_text(gl: &Context, text: &str, x: f32, y: f32, size: f32, color: Vec4, sw: f32, sh: f32) {
    let mut offset = 0.0;
    for c in text.chars() {
        if c != ' ' {
            // Рисуем кубик вместо буквы
            draw_rect(gl, x + offset, y, size * 0.8, size, color, sw, sh);
        }
        offset += size * 1.2; // Широкий пробел как в Lua (spacing)
    }
}

unsafe fn draw_lobby(gl: &Context) {
    let w = SCREEN_W;
    let h = SCREEN_H;

    // 1. Градиент фон (аналог mkGrad из Lua)
    // Верхний левый -> Верхний правый -> Нижний правый -> Нижний левый
    let c1 = vec4(0.45, 0.15, 0.80, 1.0);
    let c2 = vec4(0.55, 0.20, 0.85, 1.0);
    let c3 = vec4(0.85, 0.30, 0.65, 1.0);
    let c4 = vec4(0.80, 0.25, 0.70, 1.0);
    
    // Рисуем 2 огромных треугольника на весь экран
    gl.uniform_4_f32(1, c1.x, c1.y, c1.z, c1.w); // Пока фон однотонный для упрощения, градиент требует другого шейдера
    draw_rect(gl, 0.0, 0.0, w, h, vec4(0.55, 0.20, 0.85, 1.0), w, h); // Фиолетовый фон

    // 2. Текст "Cubic Battle" (из кубиков)
    let title_size = 40.0;
    let title_w = "Cubic Battle".len() as f32 * title_size * 1.2;
    let title_x = w/2.0 - title_w/2.0;
    draw_cubic_text(gl, "CUBIC BATTLE", title_x, h/2.0 - 150.0, title_size, vec4(1.0, 1.0, 1.0, 1.0), w, h);

    // 3. Подпись "Touch & Dodge"
    let sub_size = 15.0;
    let sub_w = "Touch & Dodge".len() as f32 * sub_size * 1.2;
    let sub_x = w/2.0 - sub_w/2.0;
    draw_cubic_text(gl, "TOUCH & DODGE", sub_x, h/2.0 - 60.0, sub_size, vec4(1.0, 1.0, 1.0, 1.0), w, h);

    // 4. Кнопка "Play"
    let btn = LOBBY_BTN;

    // Тень кнопки (love.graphics.setColor(0,0,0,0.20))
    draw_rect(gl, btn.x+5.0, btn.y+6.0, btn.w, btn.h, vec4(0.0, 0.0, 0.0, 0.2), w, h);

    // Сама кнопка (love.graphics.setColor(0.55, 0.20, 0.85, 1))
    draw_rect(gl, btn.x, btn.y, btn.w, btn.h, vec4(0.55, 0.20, 0.85, 1.0), w, h);

    // Обводка кнопки (рисуем 4 тонких прямоугольника вместо "line")
    let border = vec4(0.0, 0.0, 0.0, 1.0);
    let t = 3.4; // Толщина обводки из Lua
    draw_rect(gl, btn.x, btn.y, btn.w, t, border, w, h); // Верх
    draw_rect(gl, btn.x, btn.y + btn.h - t, btn.w, t, border, w, h); // Низ
    draw_rect(gl, btn.x, btn.y, t, btn.h, border, w, h); // Лево
    draw_rect(gl, btn.x + btn.w - t, btn.y, t, btn.h, border, w, h); // Право

    // Текст кнопки "Play"
    let play_size = 20.0;
    let play_w = "PLAY".len() as f32 * play_size * 1.2;
    let play_x = btn.x + btn.w/2.0 - play_w/2.0;
    draw_cubic_text(gl, "PLAY", play_x, btn.y + 20.0, play_size, vec4(1.0, 1.0, 1.0, 1.0), w, h);
}

// ========== MAIN ==========
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Trace));
    info!("Cubic Battle запущен на Rust!");

    let mut gl_ctx: Option<GlContext> = None;
    let mut shader_program = None;

    loop {
        app.poll_events(None, |event| {
            match event {
                PollEvent::Main(MainEvent::InitWindow { .. }) => {
                    let window = app.native_window().expect("No window");
                    unsafe {
                        if let Some(ctx) = init_egl(window.as_ptr()) {
                            gl_ctx = Some(ctx);
                            if let Some(ref gl) = gl_ctx {
                                place_lobby();
                                
                                // Простой шейдер для 2D (как в C++)
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
                                gl.use_program(prog);
                                shader_program = Some(prog);
                            }
                        }
                    }
                }
                PollEvent::Main(MainEvent::Destroy) => {
                    gl_ctx = None;
                }
                PollEvent::Input(input_event) => {
                    if let Some(motion) = input_event.as_motion_event() {
                        if motion.get_action() == 0 { // ACTION_DOWN
                            let x = motion.get_x(0) as f32;
                            let y = motion.get_y(0) as f32;
                            unsafe {
                                // Проверка нажатия на кнопку (из Lua: lobby.touchpressed)
                                if CURRENT_STATE is GameState::Lobby {
                                    let btn = LOBBY_BTN;
                                    if x >= btn.x && x <= btn.x + btn.w && y >= btn.y && y <= btn.y + btn.h {
                                        info!("Кнопка PLAY нажата! Переход в игру.");
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

        if let (Some(ref ctx), Some(_)) = (&gl_ctx, shader_program) {
            let gl = &ctx.gl;
            unsafe {
                gl.viewport(0, 0, SCREEN_W as i32, SCREEN_H as i32);
                gl.clear_color(0.1, 0.1, 0.15, 1.0);
                gl.clear(COLOR_BUFFER_BIT);
                
                match CURRENT_STATE {
                    GameState::Lobby => draw_lobby(gl),
                    GameState::Game => {
                        // Пока рисуем синий экран, если перешли в игру
                        draw_rect(gl, 0.0, 0.0, SCREEN_W, SCREEN_H, vec4(0.2, 0.6, 1.0, 1.0), SCREEN_W, SCREEN_H);
                    }
                }

                eglSwapBuffers(ctx.display, ctx.surface);
            }
        }
    }
              }
