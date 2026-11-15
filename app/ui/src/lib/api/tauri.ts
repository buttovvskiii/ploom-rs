import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Backend, CommandArgs, CommandName, CommandResult, Unsubscribe } from "./client";
import { EVENTS, type ActivityEvent, type Snapshot } from "./types";

/** Tauri's `listen` is async; the UI wants a synchronous unsubscribe. */
function subscribe<T>(event: string, handler: (payload: T) => void): Unsubscribe {
  let unlisten: UnlistenFn | null = null;
  let cancelled = false;
  listen<T>(event, (e) => handler(e.payload))
    .then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    })
    .catch((error: unknown) => console.error(`listen(${event}) failed`, error));
  return () => {
    cancelled = true;
    unlisten?.();
  };
}

export function createTauriBackend(): Backend {
  return {
    invoke<K extends CommandName>(name: K, args: CommandArgs<K>): Promise<CommandResult<K>> {
      return invoke<CommandResult<K>>(name, args as Record<string, unknown>);
    },
    onSnapshot: (handler) => subscribe<Snapshot>(EVENTS.snapshot, handler),
    onActivity: (handler) => subscribe<ActivityEvent>(EVENTS.activity, handler),
  };
}
