use atlas::Atlas;

fn main() {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        std::env::remove_var("WAYLAND_DISPLAY");
    }

    Atlas::init().run().unwrap();
}
