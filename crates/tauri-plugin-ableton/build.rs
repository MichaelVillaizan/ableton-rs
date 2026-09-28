const COMMANDS: &[&str] = &[
    "start",
    "close",
    "is_connected",
    "get_ping",
    "get_prop",
    "set_prop",
    "send_command",
    "add_listener",
    "remove_listener",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
