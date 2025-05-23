
fn main() {

    #[cfg(not(target_arch="wasm32"))]
    {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            unsafe {
                std::env::remove_var("WAYLAND_DISPLAY");
            }
        }

        unsafe {
            std::env::set_var("RUST_BACKTRACE", "1");
        }
    }

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = atlas::AtlasApp::default();
    event_loop.run_app(&mut app).unwrap();

}
