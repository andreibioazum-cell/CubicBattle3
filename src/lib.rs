mod lobby;
mod settings;

use android_activity::{AndroidApp, MainEvent, PollEvent};
use ndk::native_window::NativeWindow;

/// Список доступных сцен
pub enum Scene {
    Lobby,
    Settings,
}

#[no_mangle]
pub fn android_main(app: AndroidApp) {
    // Инициализация логов (можно смотреть через adb logcat)
    android_logger::init_once(
        android_logger::Config::default().with_min_level(log::Level::Info),
    );

    let mut scene = Scene::Lobby;
    let mut running = true;
    let mut native_window: Option<NativeWindow> = None;

    // Главный цикл приложения
    while running {
        // 1. Обработка системных событий Android (окно, жизненный цикл)
        app.poll_events(None, |event| {
            match event {
                // Когда окно создано и готово к работе
                PollEvent::Main(MainEvent::InitWindow { .. }) => {
                    native_window = app.native_window();
                }
                // Когда окно уничтожено (например, приложение свернули или закрыли)
                PollEvent::Main(MainEvent::TerminateWindow { .. }) => {
                    native_window = None;
                }
                // Выход из приложения
                PollEvent::Main(MainEvent::Destroy) => {
                    running = false;
                }
                _ => {}
            }
        });

        // 2. Рендеринг (только если окно существует)
        if let Some(window) = &native_window {
            // "Запираем" буфер окна для рисования программным способом
            if let Ok(mut buffer) = NativeWindow::lock(window, None) {
                let width = buffer.width() as usize;
                let height = buffer.height() as usize;
                let stride = buffer.stride() as usize;

                // Получаем доступ к "сырым" байтам пикселей (формат обычно RGBA8888)
                // Размер: (шаг строки * высота * 4 байта на пиксель)
                let pixels = unsafe {
                    std::slice::from_raw_parts_mut(
                        buffer.bits() as *mut u8, 
                        stride * height * 4
                    )
                };

                // 3. Вызов отрисовки текущей сцены
                match scene {
                    Scene::Lobby => {
                        // Если функция вернула true — переключаемся
                        if lobby::update_and_draw(pixels, &app, width, height, stride) {
                            scene = Scene::Settings;
                        }
                    }
                    Scene::Settings => {
                        if settings::update_and_draw(pixels, &app, width, height, stride) {
                            scene = Scene::Lobby;
                        }
                    }
                }

                // Буфер разблокируется автоматически, когда переменная `buffer` выйдет из области видимости
            }
        }
    }
}
