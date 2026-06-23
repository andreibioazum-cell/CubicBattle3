use android_activity::{AndroidApp, input::{InputEvent, MotionAction}, InputStatus};
use glow::HasContext;
use fontdue::Font;
use std::ffi::CString;

pub fn render(gl: &glow::Context, app: &AndroidApp, width: i32, height: i32) -> bool {
    unsafe {
        gl.clear_color(0.1, 0.15, 0.3, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
    }

    // Загрузка шрифта (безопасная, без unwrap паники)
    static mut FONT: Option<Font> = None;
    unsafe {
        if FONT.is_none() {
            if let Ok(filename) = CString::new("Font.ttf") {
                if let Some(mut asset) = app.asset_manager().open(&filename) {
                    if let Some(buffer) = asset.buffer() {
                        FONT = Font::from_bytes(buffer.to_vec(), fontdue::FontSettings::default()).ok();
                    }
                }
            }
        }
    }

    let mut go_settings = false;
    let (bw, bh) = (width as f32 * 0.4, height as f32 * 0.15);
    let (bx, by) = ((width as f32 - bw)/2.0, (height as f32 - bh)/2.0);

    if let Ok(mut iter) = app.input_events_iter() {
        while iter.next(|ev| {
            if let InputEvent::MotionEvent(m) = ev {
                if m.action() == MotionAction::Down {
                    let x = m.pointer_at_index(0).x();
                    let y = m.pointer_at_index(0).y();
                    if x > bx && x < bx + bw && y > by && y < by + bh { go_settings = true; }
                }
            }
            InputStatus::Handled
        }) {}
    }
    go_settings
}
