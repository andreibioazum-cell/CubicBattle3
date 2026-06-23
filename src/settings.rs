use android_activity::{AndroidApp, input::{InputEvent, MotionAction}, InputStatus};
use glow::HasContext;

pub fn render(gl: &glow::Context, app: &AndroidApp, _width: i32, _height: i32) -> bool {
    unsafe {
        gl.clear_color(0.3, 0.1, 0.1, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
    }

    let mut go_back = false;
    if let Ok(mut iter) = app.input_events_iter() {
        while iter.next(|ev| {
            if let InputEvent::MotionEvent(m) = ev {
                if m.action() == MotionAction::Down {
                    go_back = true; 
                }
            }
            InputStatus::Handled
        }) {}
    }
    go_back
}
