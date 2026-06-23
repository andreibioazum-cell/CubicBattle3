use android_activity::{AndroidApp, input::{InputEvent, MotionAction}, InputStatus};

pub fn update_and_draw(
    pixels: &mut [u8], 
    app: &AndroidApp, 
    width: usize, 
    height: usize, 
    stride: usize
) -> bool {
    // 1. Очистка экрана (Заливаем темно-синим)
    for y in 0..height {
        for x in 0..width {
            let idx = (y * stride + x) * 4;
            pixels[idx] = 25;     // R
            pixels[idx + 1] = 30; // G
            pixels[idx + 2] = 80; // B
            pixels[idx + 3] = 255; // A
        }
    }

    // 2. Рисуем "Кнопку" (Просто белый прямоугольник в центре)
    let bx = width / 2 - 150;
    let by = height / 2 - 50;
    let bw = 300;
    let bh = 100;

    for y in by..(by + bh) {
        for x in bx..(bx + bw) {
            if x < width && y < height {
                let idx = (y * stride + x) * 4;
                pixels[idx] = 200;
                pixels[idx + 1] = 200;
                pixels[idx + 2] = 255;
            }
        }
    }

    // 3. Обработка ввода
    let mut go_settings = false;
    if let Ok(mut iter) = app.input_events_iter() {
        while iter.next(|ev| {
            if let InputEvent::MotionEvent(m) = ev {
                if m.action() == MotionAction::Down {
                    let x = m.pointer_at_index(0).x() as usize;
                    let y = m.pointer_at_index(0).y() as usize;
                    if x > bx && x < bx + bw && y > by && y < by + bh {
                        go_settings = true;
                    }
                }
            }
            InputStatus::Handled
        }) {}
    }

    go_settings
}
