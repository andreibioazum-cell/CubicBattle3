use android_activity::{
    AndroidApp,
    input::{InputEvent, MotionAction},
    InputStatus,
};
use glow::HasContext;

pub fn render(gl: &glow::Context, app: &AndroidApp) -> bool {
    unsafe {
        gl.viewport(0, 0, 1280, 720);
        gl.clear_color(0.3, 0.05, 0.05, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
    }

    let mut go_back = false;

    if let Ok(mut iter) = app.input_events_iter() {
        while iter.next(|event| {
            if let InputEvent::MotionEvent(motion) = event {
                if motion.action() == MotionAction::Down {
                    go_back = true; // любой тап возвращает назад
                }
            }

            InputStatus::Handled
        }) {}
    }

    go_back
}
