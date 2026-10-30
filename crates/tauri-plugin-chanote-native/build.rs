// No JavaScript commands: the app's Rust code calls the Swift side directly.
const COMMANDS: &[&str] = &[];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).ios_path("ios").build();
}
