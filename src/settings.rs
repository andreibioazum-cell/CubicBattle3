use android_activity::{AndroidApp, input::{InputEvent, MotionAction}, InputStatus};

pub fn update_and_draw(
    pixels: &mut [u8], 
    app: &AndroidApp, 
    width: usize, 
    height: usize, 
    stride: usize
) -> bool {
    // Заливаем красным
    for y in 0..height {
        for x in 0..width {
            let idx = (y * stride + x) * 4;
            pixels[idx] = 100;
            pixels[idx + 1] = 20;
            pixels[idx + 2] = 20;
            pixels[idx + 3] = 255;
        }
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
