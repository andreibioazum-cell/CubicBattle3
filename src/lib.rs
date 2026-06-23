use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use log::info;

// Точка входа в Android приложение на Rust
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    // Настраиваем логирование, чтобы видеть сообщения в Logcat
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );

    info!("Cubic Battle запущен на Rust!");
    
    let mut window_created = false;

    // Главный цикл игры
    loop {
        // Обработка событий Android (создание окна, пауза, ввод)
        app.poll_events(
            None, // Бесконечный таймаут ожидания
            |event| {
                match event {
                    PollEvent::Main(MainEvent::InitWindow { .. }) => {
                        info!("Окно создано! Можно рисовать.");
                        window_created = true;
                    }
                    PollEvent::Main(MainEvent::Destroy) => {
                        info!("Закрытие приложения.");
                        window_created = false;
                    }
                    PollEvent::Input(input_event) => {
                        // Тут мы будем обрабатывать тач и клавиатуру
                        let _ = input_event; // Пока просто игнорируем
                    }
                    _ => {}
                }
                InputStatus::Handled
            },
        );

        if window_created {
            // Тут будет игровой цикл, обновление логики и рендер OpenGL
        }
    }
}
