import { createClient, type ApiClient } from "./client";
import { createTauriBackend } from "./tauri";

export * from "./client";
export type * from "./types";

/** Running inside the desktop shell (the IPC bridge is injected before our scripts). */
export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function loadClient(): ApiClient {
  return createClient(createTauriBackend());
}
