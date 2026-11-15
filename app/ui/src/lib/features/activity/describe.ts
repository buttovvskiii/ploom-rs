import type { ActivityEvent } from "$lib/api";
import type { IconName } from "$lib/components/icons/paths";
import { formatTime } from "$lib/i18n/format";
import { i18n } from "$lib/i18n/index.svelte";
import { targetLabel } from "$lib/modes";

export function describe(event: ActivityEvent): string {
  const t = i18n.t.feed;
  switch (event.type) {
    case "connected":
      return t.connected(event.deviceName);
    case "connectionLost":
      return t.connectionLost;
    case "disconnected":
      return t.disconnected;
    case "stateChanged":
      return t.state[event.activity];
    case "chargingChanged":
      return t.charging(event.charging);
    case "vibrationChanged":
      return t.vibration(event.on);
    case "lockChanged":
      return t.lock[event.lock];
    case "autoStartChanged":
      return t.autoStart(event.enabled);
    case "profileApplied":
      return t.profileApplied(targetLabel(event.target), event.confirmed);
    case "sessionLogReceived":
      return t.sessionLog;
    case "limitReached":
      return t.limitReached(event.sticks, formatTime(event.lockedUntilMs));
    case "cooldownEnded":
      return t.cooldownEnded;
    case "relocked":
      return t.relocked;
    case "rawFrameSent":
      return t.rawFrame(event.replies);
  }
}

export function iconFor(event: ActivityEvent): IconName {
  switch (event.type) {
    case "connected":
    case "connectionLost":
    case "disconnected":
      return "devices";
    case "stateChanged":
      return event.activity === "charging" ? "bolt" : "heating";
    case "chargingChanged":
      return "bolt";
    case "vibrationChanged":
      return "vibrate";
    case "lockChanged":
      return "lock";
    case "autoStartChanged":
      return "heating";
    case "profileApplied":
      return "frames";
    case "sessionLogReceived":
      return "history";
    case "limitReached":
    case "cooldownEnded":
    case "relocked":
      return "limit";
    case "rawFrameSent":
      return "console";
  }
}

/** Events worth a warning tint in the feed. */
export function isAlert(event: ActivityEvent): boolean {
  return (
    event.type === "connectionLost" ||
    event.type === "relocked" ||
    (event.type === "profileApplied" && !event.confirmed)
  );
}
