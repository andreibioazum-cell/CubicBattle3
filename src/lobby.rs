use android_activity::{AndroidApp, input::{InputEvent, MotionAction}};
use glow::HasContext;
use fontdue::Font;

static mut INIT: bool = false;
static mut FONT: Option<Font> = None;

pub fn render(gl: &glow::Context, app: &AndroidApp) -> bool {
    unsafe {
        if !INIT {
            let font_bytes = app.asset_manager()
                .open("Font.ttf")
                .unwrap()
                .buffer()
                .unwrap()
                .to_vec();

            FONT = Some(Font::from_bytes(font_bytes, fontdue::FontSettings::default()).unwrap());
            INIT = true;
        }

        gl.viewport(0, 0, 1280, 720);
        gl.clear_color(0.1, 0.1, 0.3, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);

        // простая кнопка Settings зона
        if let Ok(mut iter) = app.input_events_iter() {
            while iter.next(|e| {
                if let InputEvent::MotionEvent(m) = e {
                    if m.action() == MotionAction::Down {
                        let x = m.pointer_at_index(0).x();
                        let y = m.pointer_at_index(0).y();

                        if x > 500.0 && x < 780.0 && y > 300.0 && y < 380.0 {
                            return true;
                        }
                    }
                }
                android_activity::InputStatus::Handled
            }) {}
        }

        false
    }
}
