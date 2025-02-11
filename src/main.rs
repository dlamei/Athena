use atlas::Atlas;

fn main() {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        std::env::remove_var("WAYLAND_DISPLAY");
    }

    std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("atlas", log::LevelFilter::Info)
        // .filter_module("atlas::vm", log::LevelFilter::Debug)
        .format_timestamp(None)
        .init();

    Atlas::init().run().unwrap();
}
