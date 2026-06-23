use android_activity::{AndroidApp, MainEvent, PollEvent};
use glow::HasContext;
use glam::vec4;
use std::ffi::{c_void, CString};
use std::ptr;
use std::time::Instant;
use fontdue::Font;

// ================= EGL =================

#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(id: *mut c_void) -> *mut c_void;
    fn eglInitialize(d: *mut c_void, ma: *mut i32, mi: *mut i32) -> i32;
    fn eglChooseConfig(d: *mut c_void, al: *const i32, c: *mut *mut c_void, cs: i32, nc: *mut i32) -> i32;
    fn eglCreateWindowSurface(d: *mut c_void, c: *mut c_void, w: *mut c_void, al: *const i32) -> *mut c_void;
    fn eglCreateContext(d: *mut c_void, c: *mut c_void, sc: *mut c_void, al: *const i32) -> *mut c_void;
    fn eglMakeCurrent(d: *mut c_void, dr: *mut c_void, r: *mut c_void, ctx: *mut c_void) -> i32;
    fn eglQuerySurface(d: *mut c_void, s: *mut c_void, a: i32, v: *mut i32) -> i32;
    fn eglSwapBuffers(d: *mut c_void, s: *mut c_void) -> i32;
    fn eglSwapInterval(d: *mut c_void, interval: i32) -> i32;
    fn eglGetProcAddress(n: *const std::os::raw::c_char) -> *mut c_void;
}

// ================= SHADERS =================

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

uniform sampler2D u_tex;
uniform vec4 u_color;

void main() {
    float alpha = texture2D(u_tex, v_uv).r;
    gl_FragColor = vec4(u_color.rgb, alpha * u_color.a);
}
"#;

// ================= TEXT RENDERER =================

struct TextRenderer {
    texture: glow::Texture,
    width: i32,
    height: i32,
}

unsafe fn create_text_texture(
    gl: &glow::Context,
    font: &Font,
    text: &str,
    size: f32,
) -> TextRenderer {
    let mut total_width = 0;
    let mut max_height = 0;
    let mut glyphs = Vec::new();

    for c in text.chars() {
        let (metrics, bitmap) = font.rasterize(c, size);
        total_width += metrics.width as i32;
        max_height = max_height.max(metrics.height as i32);
        glyphs.push((metrics, bitmap));
    }

    let mut image = vec![0u8; (total_width * max_height) as usize];
    let mut pen_x = 0;

    for (metrics, bitmap) in glyphs {
        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let src = bitmap[y * metrics.width + x];
                let dst = ((y as i32 * total_width) + pen_x + x as i32) as usize;
                image[dst] = src;
            }
        }
        pen_x += metrics.width as i32;
    }

    let tex = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));

    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RED as i32,
        total_width,
        max_height,
        0,
        glow::RED,
        glow::UNSIGNED_BYTE,
        Some(&image),
    );

    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);

    TextRenderer {
        texture: tex,
        width: total_width,
        height: max_height,
    }
}

// ================= MAIN =================

#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default());

    let mut running = true;
    let mut last_frame = Instant::now();

    let mut gl_ctx: Option<(*mut c_void, *mut c_void)> = None;
    let mut gl: Option<glow::Context> = None;
    let mut program: Option<glow::Program> = None;
    let mut text: Option<TextRenderer> = None;

    while running {
        app.poll_events(None, |event| {
            if let PollEvent::Main(MainEvent::InitWindow { .. }) = event {
                unsafe {
                    let win = app.native_window().unwrap();
                    let display = eglGetDisplay(ptr::null_mut());
                    eglInitialize(display, ptr::null_mut(), ptr::null_mut());

                    let attrs = [0x3033, 4, 0x3038];
                    let mut config = ptr::null_mut();
                    let mut num = 0;
                    eglChooseConfig(display, attrs.as_ptr(), &mut config, 1, &mut num);

                    let surface = eglCreateWindowSurface(display, config, win.ptr().as_ptr() as *mut _, ptr::null());
                    let context = eglCreateContext(display, config, ptr::null_mut(), [0x3098, 2, 0x3038].as_ptr());

                    eglMakeCurrent(display, surface, surface, context);
                    eglSwapInterval(display, 1);

                    let mut w = 0;
                    let mut h = 0;
                    eglQuerySurface(display, surface, 0x3057, &mut w);
                    eglQuerySurface(display, surface, 0x3056, &mut h);

                    let gl_ctx_local = glow::Context::from_loader_function(|s| {
                        eglGetProcAddress(CString::new(s).unwrap().as_ptr())
                    });

                    let prog = gl_ctx_local.create_program().unwrap();
                    let vs = gl_ctx_local.create_shader(glow::VERTEX_SHADER).unwrap();
                    gl_ctx_local.shader_source(vs, VS);
                    gl_ctx_local.compile_shader(vs);

                    let fs = gl_ctx_local.create_shader(glow::FRAGMENT_SHADER).unwrap();
                    gl_ctx_local.shader_source(fs, FS);
                    gl_ctx_local.compile_shader(fs);

                    gl_ctx_local.attach_shader(prog, vs);
                    gl_ctx_local.attach_shader(prog, fs);
                    gl_ctx_local.link_program(prog);

                    let verts: [f32; 12] = [
                        0.0,0.0, 1.0,0.0, 1.0,1.0,
                        0.0,0.0, 1.0,1.0, 0.0,1.0
                    ];

                    let vbo = gl_ctx_local.create_buffer().unwrap();
                    gl_ctx_local.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                    gl_ctx_local.buffer_data_u8_slice(
                        glow::ARRAY_BUFFER,
                        std::slice::from_raw_parts(verts.as_ptr() as *const u8, 48),
                        glow::STATIC_DRAW,
                    );

                    gl_ctx_local.enable_vertex_attrib_array(0);
                    gl_ctx_local.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);

                    let font_bytes = app.asset_manager().open("Font.ttf").unwrap().buffer().unwrap().to_vec();
                    let font = Font::from_bytes(font_bytes, fontdue::FontSettings::default()).unwrap();

                    let text_renderer = create_text_texture(&gl_ctx_local, &font, "CUBIC BATTLE", 64.0);

                    gl = Some(gl_ctx_local);
                    program = Some(prog);
                    text = Some(text_renderer);
                    gl_ctx = Some((display, surface));
                }
            }

            if let PollEvent::Main(MainEvent::Destroy) = event {
                running = false;
            }
        });

        let _dt = last_frame.elapsed().as_secs_f32();
        last_frame = Instant::now();

        if let (Some((display, surface)), Some(gl), Some(program), Some(text)) =
            (&gl_ctx, &gl, &program, &text)
        {
            unsafe {
                gl.viewport(0, 0, 1280, 720);
                gl.clear_color(0.05, 0.1, 0.2, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);

                gl.use_program(Some(*program));
                gl.bind_texture(glow::TEXTURE_2D, Some(text.texture));
                gl.uniform_4_f32(
                    gl.get_uniform_location(*program, "u_color").as_ref(),
                    1.0, 1.0, 1.0, 1.0
                );

                gl.draw_arrays(glow::TRIANGLES, 0, 6);

                eglSwapBuffers(*display, *surface);
            }
        }
    }
                        }
