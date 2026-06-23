use android_activity::{
    AndroidApp,
    input::{InputEvent, MotionAction},
    InputStatus,
};
use glow::HasContext;

/// Lobby сцена
/// Возвращает true если нужно перейти в Settings
pub fn render(
    gl: &glow::Context,
    app: &AndroidApp,
    width: i32,
    height: i32,
) -> bool {
    unsafe {
        // Фон
        gl.clear_color(0.1, 0.15, 0.35, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
    }

    // ===== КНОПКА SETTINGS =====
    // Кнопка по центру экрана
    let button_width = width as f32 * 0.3;
    let button_height = height as f32 * 0.12;

    let button_x = (width as f32 - button_width) * 0.5;
    let button_y = (height as f32 - button_height) * 0.5;

    let mut go_settings = false;

    // ===== INPUT =====
    if let Ok(mut iter) = app.input_events_iter() {
        while iter.next(|event| {
            if let InputEvent::MotionEvent(motion) = event {
                if motion.action() == MotionAction::Down {
                    let x = motion.pointer_at_index(0).x();
                    let y = motion.pointer_at_index(0).y();

                    if x > button_x
                        && x < button_x + button_width
                        && y > button_y
                        && y < button_y + button_height
                    {
                        go_settings = true;
                    }
                }
            }

            InputStatus::Handled
        }) {}
    }

    go_settings
}
