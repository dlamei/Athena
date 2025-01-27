use atlas::Atlas;

fn main() {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        std::env::remove_var("WAYLAND_DISPLAY");
    }

    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("atlas", log::LevelFilter::Info)
        .format_timestamp(None)
        .init();

    // atlas::vm::run();
    Atlas::init().run().unwrap();
}
