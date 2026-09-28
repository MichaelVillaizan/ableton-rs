import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type ConnectKind = "start" | "realtime" | "heartbeat";
export type DisconnectKind = "realtime" | "heartbeat";

/** Error returned by every command of the plugin. */
export interface AbletonError {
  kind:
    | "notStarted"
    | "notConnected"
    | "connectionTimeout"
    | "closed"
    | "timeout"
    | "disconnected"
    | "command"
    | "remote"
    | "auth"
    | "protocol"
    | "decode"
    | "encode"
    | "invalidArgument"
    | "unknownListener"
    | "other";
  message: string;
  /** Python exception class for `command` errors (e.g. "RuntimeError"). */
  errorType: string | null;
}

export interface Command {
  ns: string;
  nsid?: string | null;
  name: string;
  /** An object for keyword arguments, an array for positional ones. */
  args?: Record<string, unknown> | unknown[];
  timeoutMs?: number;
  /** Route the result through the etag response cache. */
  cache?: boolean;
}

/** Starts the client (unless `autoStart` already did) and waits for Live. */
export function start(timeoutMs?: number): Promise<void> {
  return invoke("plugin:ableton|start", { timeoutMs });
}

/** Closes the client. */
export function close(): Promise<void> {
  return invoke("plugin:ableton|close");
}

export function isConnected(): Promise<boolean> {
  return invoke("plugin:ableton|is_connected");
}

/** Heartbeat round-trip time in milliseconds. */
export function getPing(): Promise<number> {
  return invoke("plugin:ableton|get_ping");
}

/** Reads a property of a Live object (e.g. `getProp("song", null, "tempo")`). */
export function getProp<T = unknown>(
  ns: string,
  nsid: string | null,
  prop: string,
  cache = false,
): Promise<T> {
  return invoke("plugin:ableton|get_prop", { ns, nsid, prop, cache });
}

/** Sets a property of a Live object. */
export function setProp(
  ns: string,
  nsid: string | null,
  prop: string,
  value: unknown,
): Promise<unknown> {
  return invoke("plugin:ableton|set_prop", { ns, nsid, prop, value });
}

/** Calls any function of a Live object. */
export function sendCommand<T = unknown>(command: Command): Promise<T> {
  return invoke("plugin:ableton|send_command", { ...command });
}

/**
 * Calls `listener` whenever a property changes. Resolves to a function that
 * removes the listener. Listeners are dropped when Live disconnects;
 * register them again in {@link onConnect}.
 */
export async function addListener<T = unknown>(
  ns: string,
  nsid: string | null,
  prop: string,
  listener: (value: T) => void,
): Promise<() => Promise<boolean>> {
  const onEvent = new Channel<T>();
  onEvent.onmessage = listener;
  const { id } = await invoke<{ id: number; eventId: string }>(
    "plugin:ableton|add_listener",
    { ns, nsid, prop, onEvent },
  );
  return () => invoke<boolean>("plugin:ableton|remove_listener", { id });
}

export function onConnect(handler: (kind: ConnectKind) => void): Promise<UnlistenFn> {
  return listen<ConnectKind>("ableton://connect", (event) => handler(event.payload));
}

export function onDisconnect(handler: (kind: DisconnectKind) => void): Promise<UnlistenFn> {
  return listen<DisconnectKind>("ableton://disconnect", (event) => handler(event.payload));
}

export function onPing(handler: (latencyMs: number) => void): Promise<UnlistenFn> {
  return listen<number>("ableton://ping", (event) => handler(event.payload));
}

export function onError(handler: (error: AbletonError) => void): Promise<UnlistenFn> {
  return listen<AbletonError>("ableton://error", (event) => handler(event.payload));
}
