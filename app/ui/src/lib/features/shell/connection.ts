import type { Snapshot } from "$lib/api";

export type ConnectionTone = "ok" | "pending" | "idle" | "error";

export function connectionTone(s: Snapshot): ConnectionTone {
  if (s.connectionError === "bluetoothUnavailable") return "error";
  switch (s.connection) {
    case "connected":
      return "ok";
    case "connecting":
    case "reconnecting":
      return "pending";
    default:
      return "idle";
  }
}
