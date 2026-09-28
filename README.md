# ableton-rs

Controla Ableton Live desde Rust. Es la traducción del cliente de
[`ableton-js`](../ableton-js/src), con arquitectura refactorizada y el mismo
protocolo: se conecta por WebSocket al **mismo Remote Script de Python**
(`ableton-js/midi-script`), sin cambios en el lado de Live.

| Crate | Qué es |
|---|---|
| [`ableton-core`](crates/ableton-core) | Cliente en Rust puro (tokio). La clase `Ableton` y todo el modelo de objetos de Live, tipado. |
| [`tauri-plugin-ableton`](crates/tauri-plugin-ableton) | Adaptador para Tauri 2: gestiona el cliente, lo expone a Rust (`app.ableton()`) y al webview (comandos + eventos). |

## Uso rápido

```rust
use ableton_core::prelude::*;

let ableton = Ableton::new(AbletonOptions::default());
ableton.start(None).await?;

let song = ableton.song();
song.set_tempo(128.0).await?;

for track in song.tracks().await? {
    for device in track.devices().await? {
        if let AnyDevice::Rack(rack) = &device {
            rack.randomize_macros().await?;
        }
    }
}

let sub = song.on_is_playing(|playing| println!("playing: {playing}")).await?;
// ...
sub.unsubscribe().await?;
ableton.close().await;
```

Hay un ejemplo completo en `cargo run -p ableton-core --example basic`.

## Equivalencias con la API de JS

| JS (`ableton-js`) | Rust (`ableton-core`) |
|---|---|
| `new Ableton({ port, password })` | `Ableton::new(AbletonOptions { port, password, ..Default::default() })` |
| `await ableton.start(5000)` | `ableton.start(Some(Duration::from_secs(5))).await?` |
| `ableton.song` | `ableton.song()` (también `session()`, `application()`, `internal()`, `midi()`) |
| `song.get("tempo")` | `song.tempo().await?` |
| `song.set("tempo", 128)` | `song.set_tempo(128.0).await?` |
| `song.addListener("tempo", cb)` | `song.on_tempo(cb).await?` → `Subscription` |
| `unsubscribe()` | `subscription.unsubscribe().await?` |
| `song.get("loop")` | `song.loop_enabled()` (`loop` es palabra reservada) |
| `device.get("type")` | `device.device_type()`; en el raw: `raw().kind` |
| `track.deleteClip(clip)` / `(id)` | `track.delete_clip(&clip)` / `("id")`: acepta objeto o id |
| `song.createMidiTrack()` (índice `-1`) | `song.create_midi_track(None)` |
| `wrapDevice` / `isRackDevice(d)` | `AnyDevice::new` / `AnyDevice::Rack(_)`, `d.as_rack()` |
| `wrapChain` / `isDrumChain(c)` | `AnyChain::new` / `AnyChain::Drum(_)` |
| `ableton.on("connect", cb)` | `ableton.events()` → `broadcast::Receiver<ClientEvent>` |
| `ableton.getProp / setProp / sendCommand` | iguales: `get_prop`, `set_prop`, `send_command` |
| `ns.get(prop, useCache)` genérico | `obj.handle().get::<T>("prop", cache)` |
| `getAvailableProperties()` | `get_available_properties()` |
| `logger: console` | [`tracing`](https://docs.rs/tracing): instala un subscriber |

Todos los namespaces de `src/ns` están traducidos: Song, SongView,
Track, TrackView, Clip, ClipSlot, Scene, Device, RackDevice, LooperDevice,
PluginDevice, DeviceView, RackDeviceView, DeviceParameter, Chain, DrumChain,
DrumPad, ChainMixerDevice, MixerDevice, Envelope, CuePoint, Groove,
GroovePool, TuningSystem, TakeLane, Application, ApplicationView, Browser,
BrowserItem, Internal, Midi y Session.

### Diferencias deliberadas

- **Tipos de Live correctos.** Donde el `.d.ts` de JS decía `number` pero Live
  usa booleanos (`metronome`, `record_mode`, `session_record`,
  `exclusive_arm`…) se usa `bool`. La decodificación es tolerante
  (acepta `0/1` y `true/false`), así que no rompe con ninguna versión.
- **Enums con respaldo.** Los strings de Live (`Quantization`, `WarpMode`…)
  son enums; un valor desconocido llega como `Unknown(String)` en vez de fallar.
- **Setters sin sentido eliminados.** `song.set("master_track" | "return_tracks" | "visible_tracks", number)`
  existían solo en los tipos de JS y no funcionaban.
- **Selección por id.** `SongView::set_selected_track(&track)`,
  `set_detail_clip`, `set_highlighted_clip_slot`, `RackDeviceView::set_selected_chain`…
  reciben objeto o id, que es lo que espera el script de Python.
- **Colores.** `Color` en todas partes (`hex()`, `rgb()`), serializado como
  número igual que en JS.
- **Ciclo de vida.** Soltar el último `Ableton` apaga las tareas;
  `close()` lo hace de forma ordenada, vaciando la cola antes del frame de cierre.

## Arquitectura de `ableton-core`

```
src/
├── client/            # la clase Ableton
│   ├── mod.rs         # fachada pública + estado compartido, start/close, auth, heartbeat
│   ├── connection.rs  # tarea dueña del WebSocket: conecta, lee, reconecta con backoff
│   ├── commands.rs    # batcher (une comandos concurrentes en un envelope), timeouts, enrutado de mensajes
│   └── listeners.rs   # registro eventId → callbacks
├── protocol.rs        # formato del wire (Command, CommandEnvelope, slots de resultado)
├── cache.rs           # caché LRU+TTL por etag
├── auth.rs            # HMAC-SHA256 del salt
├── wire.rs            # FromWire: JSON de Live → tipos Rust (incluye objetos Live)
├── macros.rs          # live_object! / props! / string_enum!: el modelo es declarativo
├── ns/                # un archivo por objeto de Live
└── types/             # Color, Note
```

Cada propiedad se declara **una vez**, con los flags que la generan:

```rust
props! { Track {
    name: String [get set observe];         // name(), set_name(), on_name()
    devices: Vec<AnyDevice> [cached observe];
    input_routing_type: RoutingType [get set(RoutingSelector) observe];
}}
```

Las tareas internas (conexión, batcher, heartbeat) solo guardan referencias
débiles al estado, así que no hay ciclos ni fugas.

## Plugin de Tauri

```rust
// src-tauri/src/lib.rs
use tauri_plugin_ableton::AbletonExt;

tauri::Builder::default()
    .plugin(tauri_plugin_ableton::init())
    .setup(|app| {
        let ableton = app.ableton().clone(); // API tipada completa en Rust
        tauri::async_runtime::spawn(async move {
            ableton.wait_for_connection().await.ok();
            let tempo = ableton.song().tempo().await;
            println!("{tempo:?}");
        });
        Ok(())
    })
```

`src-tauri/capabilities/default.json`:

```json
{ "permissions": ["core:default", "ableton:default"] }
```

Configuración opcional en `tauri.conf.json` (todo en ms):

```json
{
  "plugins": {
    "ableton": {
      "port": 39031,
      "password": "secret",
      "commandTimeoutMs": 5000,
      "autoStart": true
    }
  }
}
```

También se puede configurar desde código con
`tauri_plugin_ableton::Builder::new().options(AbletonOptions { .. }).auto_start(false).build()`.

### Desde el frontend

`crates/tauri-plugin-ableton/guest-js` contiene los bindings (`npm run build`
genera `dist-js`):

```ts
import { getProp, setProp, sendCommand, addListener, onConnect } from "tauri-plugin-ableton-api";

const tempo = await getProp<number>("song", null, "tempo");
await setProp("song", null, "tempo", 128);
await sendCommand({ ns: "song", name: "start_playing" });

const stop = await addListener<number>("song", null, "tempo", (t) => console.log(t));
await onConnect((kind) => console.log("Live conectado", kind));
```

Comandos: `start`, `close`, `is_connected`, `get_ping`, `get_prop`, `set_prop`,
`send_command`, `add_listener`, `remove_listener`. Eventos:
`ableton://connect`, `ableton://disconnect`, `ableton://ping`, `ableton://error`.

## Desarrollo

```sh
cargo test --workspace     # unit + integración contra un Remote Script falso
cargo clippy --workspace --all-targets
```

Los tests de integración (`crates/ableton-core/tests`) levantan un servidor
que imita a `Socket.py`: saludo `connect`, auth HMAC, batching, caché por etag,
listeners, errores con `errorType`, timeouts y reconexión.

La versión del Remote Script esperada está en `ableton_core::PLUGIN_VERSION`
(`5.0.3`). Al conectar se avisa si la del plugin instalado no coincide.
