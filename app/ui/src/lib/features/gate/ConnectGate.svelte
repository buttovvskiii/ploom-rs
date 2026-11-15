<script lang="ts" module>
  // At launch the backend may not have begun its first attempt yet; hold "searching" briefly.
  let launched = false;
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import type { DiscoveredDevice } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import SegmentedControl from "$lib/components/SegmentedControl.svelte";
  import SignalBars from "$lib/components/SignalBars.svelte";
  import Spinner from "$lib/components/Spinner.svelte";
  import Icon from "$lib/components/icons/Icon.svelte";
  import DeviceViewer from "$lib/features/device3d/DeviceViewer.svelte";
  import { i18n, LOCALES, type Locale } from "$lib/i18n/index.svelte";
  import { fade } from "$lib/motion";
  import { run } from "$lib/state/api.svelte";
  import { prefs } from "$lib/state/prefs.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";

  const t = $derived(i18n.t);
  const snap = $derived(snapshot.current);

  let scanning = $state(false);
  let found = $state<DiscoveredDevice[] | null>(null);
  let pendingId = $state<string | null>(null);

  const LAUNCH_GRACE_MS = 1500;
  const DWELL_MS = 350;
  let warming = $state(!launched);

  type View = "nobt" | "searching" | "choose" | "notfound" | "idle";
  const target = $derived.by((): View => {
    if (snap.connectionError === "bluetoothUnavailable") return "nobt";
    if (scanning || snap.scanning || snap.connection === "connecting") return "searching";
    if (snap.connection === "reconnecting") {
      return snap.connectionError === null ? "searching" : "notfound";
    }
    if (found && found.length > 1) return "choose";
    if (found && found.length === 0) return "notfound";
    return warming ? "searching" : "idle";
  });

  // What is on screen follows `target` only once it has held still, so states never blink.
  let view = $state<View>("searching");
  $effect(() => {
    const next = target;
    if (next === view) return;
    const timer = setTimeout(() => (view = next), DWELL_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (snap.connection !== "idle") {
      launched = true;
      warming = false;
    }
  });

  onMount(() => {
    if (!warming) return;
    const timer = setTimeout(() => {
      launched = true;
      warming = false;
    }, LAUNCH_GRACE_MS);
    return () => clearTimeout(timer);
  });

  async function search(): Promise<void> {
    if (scanning) return;
    scanning = true;
    found = null;
    // Stop the background retries so the scan has the radio to itself.
    if (snap.connection !== "idle") await run((api) => api.disconnect());
    const result = await run((api) => api.scan());
    scanning = false;
    if (!result) return;
    const sorted = [...result].sort((a, b) => (b.rssiDbm ?? -999) - (a.rssiDbm ?? -999));
    const only = sorted.length === 1 ? sorted[0] : undefined;
    if (only) {
      await connect(only.id);
    } else {
      found = sorted;
    }
  }

  async function connect(id: string): Promise<void> {
    pendingId = id;
    await run((api) => api.connect({ id }));
    pendingId = null;
    found = null;
  }
</script>

<div class="gate">
  <div class="titlebar" data-tauri-drag-region></div>

  <main class="center">
    <div class="stage" aria-hidden="true">
      <DeviceViewer snapshot={snap} finish={prefs.finishFor(null)} />
    </div>

    <div class="copy" aria-live="polite">
      {#key view}
        <div class="state" in:fade={{ duration: 260, delay: 110 }} out:fade={{ duration: 110 }}>
          {#if view === "nobt"}
            <h1>{t.disconnected.bluetoothTitle}</h1>
            <p class="body">{t.disconnected.bluetoothBody}</p>
          {:else if view === "choose"}
            <h1>{t.gate.choose}</h1>
            <ul class="devices">
              {#each found ?? [] as device (device.id)}
                <li>
                  <button
                    type="button"
                    class="device"
                    disabled={pendingId !== null}
                    aria-label={t.gate.connectTo(device.name)}
                    onclick={() => connect(device.id)}
                  >
                    <span class="device-name">{device.name}</span>
                    {#if pendingId === device.id}
                      <Spinner size={14} />
                    {:else}
                      <SignalBars
                        rssi={device.rssiDbm}
                        label={device.rssiDbm === null
                          ? t.gate.signalUnknown
                          : t.devices.signal(device.rssiDbm)}
                      />
                      <Icon name="chevronRight" size={14} />
                    {/if}
                  </button>
                </li>
              {/each}
            </ul>
          {:else}
            <h1>{view === "notfound" ? t.gate.notFoundTitle : t.gate.title}</h1>
            <p class="body">
              {#if view === "searching"}
                <span class="status"
                  ><Spinner size={13} />{snap.connection === "connecting" && !scanning
                    ? t.gate.connecting
                    : t.gate.searching}</span
                >
              {:else if view === "notfound"}
                {found ? t.gate.foundNone : t.gate.notFoundBody}
              {:else}
                {t.gate.body}
              {/if}
            </p>
          {/if}

          {#if view !== "nobt" && view !== "choose"}
            <ul class="hints">
              <li>{t.disconnected.hintWake}</li>
              <li>{t.disconnected.hintExclusive}</li>
            </ul>
          {/if}

          {#if view === "idle" || view === "notfound" || view === "choose"}
            <div class="actions">
              <Button
                variant={view === "choose" ? "secondary" : "primary"}
                size="large"
                onclick={search}
              >
                {view === "idle" ? t.gate.search : t.gate.searchAgain}
              </Button>
            </div>
          {/if}
        </div>
      {/key}
    </div>
  </main>

  <div class="language">
    <SegmentedControl
      size="small"
      label={t.nav.language}
      options={LOCALES.map((l) => ({ value: l, label: l === "en" ? "English" : "Русский" }))}
      value={i18n.locale}
      onchange={(l: Locale) => i18n.set(l)}
    />
  </div>
</div>

<style>
  .gate {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    background: var(--window);
  }
  .titlebar {
    height: 46px;
    flex: none;
  }
  .center {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: clamp(24px, 5vw, 56px);
    padding: 0 40px 24px;
  }
  .stage {
    flex: none;
    --h: clamp(260px, 52vh, 400px);
    height: var(--h);
    width: calc(var(--h) * 0.66);
  }
  .copy {
    width: min(380px, 100%);
    min-height: 250px;
    display: grid;
    align-items: center;
  }
  /* Outgoing and incoming states share one cell while they cross-fade. */
  .state {
    grid-area: 1 / 1;
    min-width: 0;
  }
  h1 {
    font: 600 28px/1.15 var(--font);
    letter-spacing: -0.02em;
  }
  .body {
    margin-top: 10px;
    min-height: 20px;
    font: 400 14px/1.4 var(--font);
    color: var(--label-2);
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 9px;
  }
  .hints {
    margin: 22px 0 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font: var(--text-callout);
    color: var(--label-2);
  }
  .hints li {
    position: relative;
    padding-left: 14px;
  }
  .hints li::before {
    content: "";
    position: absolute;
    left: 2px;
    top: 0.62em;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--label-3);
  }
  .actions {
    margin-top: 26px;
  }
  .devices {
    list-style: none;
    margin: 16px 0 0;
    padding: 0;
    border-radius: var(--radius-group);
    background: var(--group);
    box-shadow: var(--surface-edge);
    overflow: hidden;
  }
  .devices li + li {
    box-shadow: inset 0 0.5px 0 var(--hairline-strong);
  }
  .device {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 46px;
    padding: 0 var(--row-x);
    border: 0;
    background: transparent;
    color: var(--label-2);
    text-align: left;
    transition: background-color var(--dur-fast) ease;
  }
  .device:hover:not(:disabled) {
    background: var(--group-hover);
  }
  .device-name {
    flex: 1;
    min-width: 0;
    color: var(--label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .language {
    position: absolute;
    left: 16px;
    bottom: 16px;
    width: 184px;
  }
  @media (max-width: 720px) {
    .center {
      flex-direction: column;
      gap: 12px;
    }
    .stage {
      --h: 200px;
    }
    .copy {
      min-height: 0;
      text-align: center;
      justify-items: center;
    }
    .hints {
      text-align: left;
    }
  }
</style>
