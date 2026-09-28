# Sincronizar ableton-rs con ableton-js

Contexto para una IA (o una persona) que tenga que llevar a este repo los cambios
de `../ableton-js`. La idea es **no releer ninguno de los dos proyectos**: se lee
el diff de ableton-js, se busca aquí cómo se mapea cada archivo y cada construcción
y solo se abren los archivos Rust afectados.

## 1. Línea base

| | |
|---|---|
| Commit de ableton-js traducido | `69de331e7fd6d987a3554a64e69f31eb2b996328` (2026-09-22) |
| Versión de ableton-js / Remote Script | `5.0.3` → `ableton_core::PLUGIN_VERSION` en `crates/ableton-core/src/lib.rs` |

**Al terminar una sincronización, actualiza esta tabla** con el nuevo commit y la
nueva versión.

## 2. Procedimiento

```sh
# 1. Qué cambió (resumen y diff; ignora tests, build y docs)
git -C ../ableton-js log --oneline 69de331..HEAD
git -C ../ableton-js diff --stat 69de331 HEAD -- src midi-script ':!*.spec.ts' ':!src/util/tests'
git -C ../ableton-js diff 69de331 HEAD -- src midi-script ':!*.spec.ts' ':!src/util/tests'

# 2. Aplica cada hunk según las secciones 3–5

# 3. Verifica
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```

Sobre `midi-script/`: el lado Python no se traduce, pero su diff indica cambios de
protocolo (sección 6) o funciones nuevas o renombradas que el TS quizá aún no expone.

Qué ignorar: `*.spec.ts`, `src/util/tests/`, `src/util/logger.ts` (aquí se usa
`tracing`), `midi-script/static/`, `vite.config.ts`, `package.json` (salvo la versión),
`CHANGELOG.md`, `hooks/`.

## 3. Mapa de archivos

Rutas Rust relativas a `crates/ableton-core/src/`.

### Cliente (`src/index.ts`)

| Parte de `index.ts` | Rust |
|---|---|
| `interface Command` / `CommandEnvelope` / `CommandSlotResult`, `truncateCommandArgs`, `summarizeCommands` | `protocol.rs` (`Command`, `CommandEnvelope::summary`, `Display for Command`) |
| `interface Response`, `handleIncoming` | `client/commands.rs` → `Inner::handle_incoming` |
| `sendCommand`, `flushCommandQueue`, `pLimit(200)`, `sendCommandEnvelope`, `msgMap`, timeouts | `client/commands.rs` (`run_batcher`, `flush_batch`, `send_envelope`, `split_results`, `PendingReply`) |
| `connectSocket`, `teardownSocket`, `closeCurrentSocket`, `scheduleReconnect`, backoff 250→2000 ms, `connectTimeoutMs` | `client/connection.rs` (`run`, `run_socket`, `Control`) |
| `start`, `close`, `waitForConnection`, `handleConnect`, `handleDisconnect`, `settleConnectionWaiters`, heartbeat, chequeo de versión | `client/mod.rs` |
| `handleServerConnect`, `abortAuthentication`, auth | `client/mod.rs` (`handle_server_connect`, `authenticate`, `abort_authentication`) + `auth.rs` |
| `sendCachedCommand`, clave de caché | `client/mod.rs` (`send_cached_command`) + `cache.rs` (`ResponseCache::key`) |
| `getProp`, `setProp`, `addPropListener`, `removePropListener`, `removeAllPropListeners`, `sendRaw`, `getPing`, `isConnected` | `client/mod.rs` (`impl Ableton`) + `client/listeners.rs` |
| `AbletonOptions` + defaults | `options.rs` **y** `crates/tauri-plugin-ableton/src/config.rs` (espejo en camelCase, en ms) |
| `EventMap` (`connect`, `disconnect`, `ping`, `error`, `message`, `raw_message`) | `events.rs` (`ClientEvent`) **y** el reenvío en `crates/tauri-plugin-ableton/src/lib.rs` (`forward_events`) |
| `TimeoutError`, `DisconnectError`, `CommandError` | `error.rs` (`Error::Timeout`, `Disconnected`, `Command`) **y** `kind()` en `crates/tauri-plugin-ableton/src/error.rs` |
| `this.song`, `session`, `application`, `internal`, `midi` | métodos `Ableton::song()` y siguientes en `client/mod.rs` |

### Namespaces (`src/ns/*.ts` → `ns/*.rs`, mismo nombre en snake_case)

| TS | Rust | Tipo en `live_object!` | `ns` |
|---|---|---|---|
| `index.ts` (`Namespace`) | `ns/mod.rs` (`Handle`, `LiveObject`, `IntoLiveId`, `index_or_end`) + `macros.rs` | — | — |
| `application.ts` | `application.rs` | singleton | `application` |
| `application-view.ts` | `application_view.rs` | singleton | `application-view` |
| `browser.ts` | `browser.rs` | singleton | `browser` |
| `browser-item.ts` | `browser_item.rs` | object | `browser-item` |
| `chain.ts` | `chain.rs` (`chain_members!`) | object | `chain` |
| `chain-mixer-device.ts` | `chain_mixer_device.rs` | object | `chain-mixer-device` |
| `clip.ts` | `clip.rs` | object | `clip` |
| `clip-slot.ts` | `clip_slot.rs` | object | `clip_slot` |
| `cue-point.ts` | `cue_point.rs` | object | `cue-point` |
| `device.ts` | `device.rs` (`device_members!`, `AnyDevice`) | object | `device` |
| `device-parameter.ts` | `device_parameter.rs` | object | `device-parameter` |
| `device-view.ts` | `device_view.rs` | view | `device-view` |
| `drum-chain.ts` | `drum_chain.rs` (`AnyChain`) | object | `drum-chain` |
| `drum-pad.ts` | `drum_pad.rs` | object | `drum-pad` |
| `envelope.ts` | `envelope.rs` | object | `envelope` |
| `groove.ts` | `groove.rs` | object | `groove` |
| `groove-pool.ts` | `groove_pool.rs` | singleton | `groove-pool` |
| `internal.ts` | `internal.rs` | singleton | `internal` |
| `looper-device.ts` | `looper_device.rs` | object | `looper-device` |
| `midi.ts` | `midi.rs` | singleton | `midi` |
| `mixer-device.ts` | `mixer_device.rs` | object | `mixer-device` |
| `plugin-device.ts` | `plugin_device.rs` | object | `plugin-device` |
| `rack-device.ts` | `rack_device.rs` | object | `rack-device` |
| `rack-device-view.ts` | `rack_device_view.rs` | view | `rack-device-view` |
| `scene.ts` | `scene.rs` | object | `scene` |
| `session.ts` | `session.rs` | singleton | `session` |
| `song.ts` | `song.rs` | singleton | `song` |
| `song-view.ts` | `song_view.rs` | singleton | `song-view` |
| `take-lane.ts` | `take_lane.rs` | object | `take-lane` |
| `track.ts` | `track.rs` | object | `track` |
| `track-view.ts` | `track_view.rs` | view | `track-view` |
| `tuning-system.ts` | `tuning_system.rs` | object | `tuning-system` |

Un namespace **nuevo** necesita: un archivo en `ns/`, las líneas `mod x;` y
`pub use x::*;` en `ns/mod.rs` y, si cuelga de `Ableton` o de otro objeto, un
método accesor (p. ej. `Song::view()`).

### Utilidades (`src/util`)

| TS | Rust |
|---|---|
| `color.ts` | `types/color.rs` |
| `note.ts` | `types/note.rs` (`Note` viaja como tupla vía serde `from`/`into`) |
| `cache.ts` | `cache.rs` |
| `hmac-sha256.ts` | `auth.rs` (los vectores de prueba de su spec están en los tests) |
| `package-version.ts` | `PLUGIN_VERSION` en `lib.rs` |
| `event-emitter.ts`, `logger.ts` | no aplica (`tokio::sync::broadcast` y `tracing`) |

## 4. Reglas de traducción de un namespace

Cada clase TS tiene cuatro interfaces. En Rust hay **una línea por propiedad**
en `props! { Tipo { ... }}` con flags:

| En TS | Flag |
|---|---|
| clave en `GettableProperties` | `get` |
| … y además `true` en `cachedProps` | `cached` (en lugar de `get`) |
| clave en `SettableProperties` con el mismo tipo | `set` |
| … con otro tipo | `set(OtroTipo)` |
| clave en `ObservableProperties` | `observe` (o `observe(OtroTipo)` si el callback recibe otro tipo) |
| entrada en `transformers` | el tipo de la línea es el envoltorio (`Track`, `Vec<Clip>`, `Option<AnyDevice>`, `Color`…); no hay transformador aparte |
| nombre de propiedad que es palabra reservada en Rust | `metodo = "nombre_wire": T [...]` |

Ejemplo: `tempo` está en Gettable, Settable y Observable, así que la línea es
`tempo: f64 [get set observe];`.

**Tipos TS → Rust**

| TS | Rust |
|---|---|
| `number`: índices, contadores, notas MIDI | `i32` (argumentos de índice: `usize`) |
| `number`: tiempos, valores, tempo, ganancia | `f64` |
| `number` cuando Live realmente usa bool | `bool` (la decodificación acepta ambos) |
| color numérico transformado a `Color` | `Color` (nullable → `Option<Color>`) |
| `string` | `String` (en setters se acepta `impl Into<String>`) |
| `X \| null` | `Option<X>` |
| `X[]` | `Vec<X>` |
| unión de strings (`"a" \| "b"`) | `string_enum!` con variante de respaldo `Unknown(String)` |
| `interface` de datos planos | `struct` serde + `crate::wire::wire_via_serde!(Tipo);` |
| `RawX` | `struct RawX` con `#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)] #[serde(default)]`; el campo `type` se llama `kind` con `#[serde(rename = "type")]` |
| `any` / `unknown` | `serde_json::Value` |
| `wrapDevice` | `AnyDevice` (añadir variante si aparece una subclase nueva) |
| `wrapChain` | `AnyChain` |

**Métodos**: `public async fooBar(a, b)` se convierte en
`pub async fn foo_bar(&self, a, b) -> Result<T>`, cuyo cuerpo es
`self.handle.call("foo_bar", json!(...)).await`.

- Hay que copiar **exactamente** la forma de los argumentos del TS: `[a, b]` va como
  `json!([a, b])` (posicional) y `{ a, b }` como `json!({ "a": a, "b": b })` (keyword).
  El Python los trata distinto.
- `sendCachedCommand` se traduce con `self.handle.call_cached(...)`.
- Un `timeout` explícito se traduce con `self.handle.call_with_timeout(...)`.
- Parámetros `clipOrId: Clip | string` → `impl IntoLiveId`.
- Default `index = -1` → `Option<usize>` + `index_or_end(index)`.
- Tipo de retorno: `()` si el TS no usa el resultado, el tipo envoltorio si hace
  `new X(...)` y `Value` si es `any`.

**Herencia**: las propiedades comunes de dispositivos van en `device_members!` (`device.rs`)
y las de cadenas en `chain_members!` (`chain.rs`). Un cambio en `device.ts` o `chain.ts`
suele ir en esas macros; revisa si la subclase lo redefine.

## 5. Desviaciones deliberadas (no las «corrijas» al sincronizar)

Si un diff toca alguna de estas, conserva el criterio Rust y adapta el cambio:

- **song**: `loop` → `loop_enabled`; `appointed_device` devuelve `Option<AnyDevice>` y
  su setter es manual (`set_appointed_device(impl IntoLiveId)`). No existen los setters de
  `master_track`, `return_tracks` y `visible_tracks` (son errores de tipado de JS).
  Usan `bool`: `metronome`, `record_mode`, `session_record`, `session_automation_record`,
  `select_on_launch`, `back_to_arranger`, `exclusive_arm`, `exclusive_solo`.
  `get_current_smpte_song_time` envía la clave `timeFormat` (camelCase, igual que en JS).
- **song_view**: los setters por id (`detail_clip`, `highlighted_clip_slot`,
  `selected_scene`, `selected_track`) son manuales, porque Python hace `get_obj(id)`.
  `selected_chain` es `Option<AnyChain>`.
- **track**: los setters de `input_/output_routing_channel/type` reciben
  `RoutingSelector` (nombre u objeto). `mixer_device` usa `get` (JS no la cachea).
  `raw().kind` corresponde a `type`.
- **track_view**: `device_insert_mode` solo tiene `set` (Live lo reporta como bool).
- **device** y familia: `type` → `device_type()`. Las cadenas de `RackDevice` son `Vec<AnyChain>`.
- **chain**: `color` es `Option<Color> [get set(Color) observe]`. `RawDrumChain` es un
  alias de `RawChain`.
- **clip**: `warp_markers` usa `observe(Value)`; `remove_notes` está `#[deprecated]`;
  `duplicate_region(.., pitch: Option<u8>, transposition_amount)`.
- **clip_slot**: `color` es `Option<Color>`; `create_clip` devuelve `()`.
- **scene**: no existe el setter de `clip_slots`.
- **rack_device_view**: setters manuales `set_selected_chain`, `clear_selected_chain` y
  `set_selected_drum_pad`.
- **plugin_device**: `get_parameter_names(begin, end: Option<usize>)`.
- **application**: `show_on_the_fly_message(msg, ShowOnTheFlyMessageOptions)`, con el
  timeout (60 s por defecto) dentro del struct.
- **midi**: `MidiMessage: TryFrom<RawMidiMessage>`; `midi_outputs` solo tiene `set`.
- **browser**: `full_refresh: () [observe]`.
- **Cliente**:
  - `on(...)` se sustituye por `events()` (canal broadcast).
  - El logger se sustituye por `tracing`.
  - La autenticación se envía en un envelope propio, fuera del batcher.
  - Las tareas guardan `Weak<Inner>`.
  - `ClientEvent` y `Error` son `#[non_exhaustive]`.

## 6. Cambios que afectan a más de un sitio

| Si en ableton-js cambia… | Toca también… |
|---|---|
| `package.json` → versión | `PLUGIN_VERSION` (lib.rs) y la línea base de este archivo |
| `AbletonOptions` | `options.rs`, `plugin/src/config.rs` (+ su test), la sección de configuración del README |
| `EventMap` | `events.rs`, `forward_events` del plugin, `plugin/src/lib.rs::events`, `guest-js/index.ts` |
| clases de error | `error.rs`, `kind()` del plugin, `AbletonError.kind` en `guest-js/index.ts` |
| formato del wire (`Command`, `Response`, `connect` con `requiresAuth`/`salt`, `__cached`, `ok`/`errorType`) | `protocol.rs`, `client/commands.rs`, `cache.rs`, `tests/support/mod.rs` (servidor falso) |
| semántica de `add_listener`/`remove_listener` (args `prop`, `nsid`, `eventId`) | `Ableton::add_prop_listener` / `remove_prop_listener` + `listener_args` |
| un método público nuevo en `Ableton` | `client/mod.rs`; si el frontend lo necesita, un comando en el plugin (`commands.rs`, `build.rs` COMMANDS, `permissions/default.toml`, `generate_handler!` en `lib.rs`, `guest-js/index.ts`) |
| propiedades o métodos de un namespace | solo su `ns/*.rs` (el plugin es genérico y no cambia) |

## 7. Comprobaciones al terminar

- [ ] `cargo test --workspace` y `cargo clippy --workspace --all-targets` sin warnings.
- [ ] Si cambió el protocolo, el servidor falso (`crates/ableton-core/tests/support/mod.rs`) lo refleja y hay un test.
- [ ] Tabla de equivalencias del `README.md` al día si cambió la API pública.
- [ ] Sección 1 de este archivo con el nuevo commit y la nueva versión.
- [ ] Nuevas desviaciones deliberadas anotadas en la sección 5.
