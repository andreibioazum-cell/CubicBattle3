use android_activity::{AndroidApp, input::{InputEvent, MotionAction}};
use glow::HasContext;

pub fn render(gl: &glow::Context, app: &AndroidApp) -> bool {
    unsafe {
        gl.viewport(0, 0, 1280, 720);
        gl.clear_color(0.3, 0.05, 0.05, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);

        if let Ok(mut iter) = app.input_events_iter() {
            while iter.next(|e| {
                if let InputEvent::MotionEvent(m) = e {
                    if m.action() == MotionAction::Down {
                        return true; // любой тап возвращает назад
                    }
                }
                android_activity::InputStatus::Handled
            }) {}
        }

        false
    }
}
