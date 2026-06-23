mod lobby;
mod settings;

use android_activity::{AndroidApp, MainEvent, PollEvent};
use ndk::native_window::NativeWindow;
use std::ptr;

pub enum Scene { Lobby, Settings }

#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default());
    
    let mut scene = Scene::Lobby;
    let mut running = true;
    let mut native_window: Option<NativeWindow> = None;

    while running {
        app.poll_events(None, |event| {
            match event {
                PollEvent::Main(MainEvent::InitWindow { .. }) => {
                    native_window = app.native_window();
                }
                PollEvent::Main(MainEvent::Destroy) => {
                    running = false;
                }
                _ => {}
            }
        });

        if let Some(window) = &native_window {
            // 1. Лочим буфер экрана
            let mut buffer = NativeWindow::lock(window, None).expect("Could not lock window");
            
            let width = buffer.width() as usize;
            let height = buffer.height() as usize;
            let stride = buffer.stride() as usize;
            
            // 2. Получаем доступ к пикселям (массив u8: [R, G, B, A, ...])
            let pixels = unsafe {
                std::slice::from_raw_parts_mut(buffer.bits() as *mut u8, stride * height * 4)
            };

            // 3. Рисуем сцену
            match scene {
                Scene::Lobby => {
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

            // 4. Разлочиваем и выводим на экран
            // Автоматически происходит при выходе buffer из области видимости (Drop)
        }
    }
}
