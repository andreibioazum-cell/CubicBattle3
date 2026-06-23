use glow::HasContext;

pub fn render(gl: &glow::Context) -> bool {
    unsafe {
        gl.clear_color(0.1, 0.1, 0.3, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
    }

    // Возвращаем true если нажата кнопка Settings
    false
}
