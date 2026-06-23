use android_activity::{AndroidApp, MainEvent, PollEvent};
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
        // Обработка событий Android (создание окна, пауза и т.д.)
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
                    _ => {}
                }
                // В новой версии callback ничего не возвращает (просто убрали return)
            },
        );

        if window_created {
            // Тут будет игровой цикл, обновление логики и рендер OpenGL
            // Пока что мы просто "держим" окно открытым
        }
    }
}
