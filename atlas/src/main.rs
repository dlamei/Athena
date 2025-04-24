use atlas::Atlas;
use glam::Vec3;

fn main() {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
        }
    }

    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("atlas", log::LevelFilter::Info)
        .filter_module("wgpu_hal::auxil::dxgi", log::LevelFilter::Error)
        .format_timestamp(None)
        .init();

    Atlas::init().run().unwrap();
}
