# ableton-rs

Traducción a Rust del cliente de `../ableton-js` (`ableton-core`) más un plugin de
Tauri 2 que lo adapta (`tauri-plugin-ableton`). Visión general en `README.md`.

Para llevar cambios de ableton-js a este repo, sigue el mapa y las reglas de:

@SYNC.md

Verificación: `cargo fmt --all && cargo clippy --workspace --all-targets && cargo test --workspace`.
