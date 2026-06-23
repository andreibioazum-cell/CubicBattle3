use android_activity::{
    AndroidApp,
    input::{InputEvent, MotionAction},
    InputStatus,
};
use glow::HasContext;
use fontdue::Font;
use std::ffi::CString;

pub fn render(gl: &glow::Context, app: &AndroidApp) -> bool {
    unsafe {
        gl.viewport(0, 0, 1280, 720);
        gl.clear_color(0.1, 0.1, 0.3, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
    }

    // ===== Загрузка шрифта (один раз через static mut) =====
    static mut FONT: Option<Font> = None;

    unsafe {
        if FONT.is_none() {
            let filename = CString::new("Font.ttf").unwrap();
            
            // 👇 ДОБАВИЛ `mut` СЮДА
            let mut asset = app.asset_manager().open(&filename).unwrap();
            
            let buffer = asset.buffer().unwrap().to_vec();

            FONT = Some(
                Font::from_bytes(buffer, fontdue::FontSettings::default()).unwrap()
            );
        }
    }

    // ===== Проверка нажатия кнопки Settings =====

    let mut go_settings = false;

    if let Ok(mut iter) = app.input_events_iter() {
        while iter.next(|event| {
            if let InputEvent::MotionEvent(motion) = event {
                if motion.action() == MotionAction::Down {
                    let x = motion.pointer_at_index(0).x();
                    let y = motion.pointer_at_index(0).y();

                    // Кнопка по центру (зона 500-780 x, 300-380 y)
                    if x > 500.0 && x < 780.0 && y > 300.0 && y < 380.0 {
                        go_settings = true;
                    }
                }
            }

            InputStatus::Handled
        }) {}
    }

    go_settings
}
