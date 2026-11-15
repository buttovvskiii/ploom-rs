<script lang="ts">
  import SegmentedControl from "$lib/components/SegmentedControl.svelte";
  import DeviceGlyph from "$lib/features/device3d/DeviceGlyph.svelte";
  import Icon from "$lib/components/icons/Icon.svelte";
  import type { IconName } from "$lib/components/icons/paths";
  import { i18n, LOCALES, type Locale } from "$lib/i18n/index.svelte";
  import { prefs, type Section } from "$lib/state/prefs.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";
  import { connectionTone } from "./connection";

  const t = $derived(i18n.t);

  const TONE_COLOR: Record<string, string> = {
    ok: "var(--green)",
    pending: "var(--orange)",
    error: "var(--red)",
    idle: "var(--label-3)",
  };

  const primary: { id: Section; icon: IconName }[] = [
    { id: "overview", icon: "overview" },
    { id: "heating", icon: "heating" },
    { id: "limit", icon: "limit" },
    { id: "history", icon: "history" },
    { id: "diagnostics", icon: "diagnostics" },
    { id: "devices", icon: "devices" },
  ];

  const snap = $derived(snapshot.current);
  const tone = $derived(connectionTone(snap));
  const finish = $derived(prefs.finishFor(snap.device));
  const deviceName = $derived(snap.device?.name ?? t.shell.noDevice);
  // The glyph's light slit mirrors the device: mode tint while heating, green while charging, a
  // quiet white when it is simply connected.
  const slitTone = $derived.by(() => {
    if (tone !== "ok") return null;
    const { activity, heatingMode, charging } = snap.status;
    if (activity === "preheating" || activity === "heating") {
      return heatingMode ? `var(--tint-${heatingMode})` : "var(--orange)";
    }
    if (charging) return "var(--green)";
    return "rgba(255, 255, 255, 0.92)";
  });

  // One selection pill slides between the entries instead of each one switching its own background
  // on and off.
  let nav: HTMLElement | undefined = $state();
  const items: Partial<Record<Section, HTMLButtonElement>> = $state({});
  let pillY = $state(0);
  let pillShown = $state(false);
  let pillArmed = $state(false);

  $effect(() => {
    const el = items[prefs.section];
    if (!el || !nav) return;
    pillY = el.getBoundingClientRect().top - nav.getBoundingClientRect().top + nav.scrollTop;
    pillShown = true;
    // Animate only after the first placement, so it does not fly in on load.
    if (!pillArmed) requestAnimationFrame(() => (pillArmed = true));
  });

  const stateLine = $derived.by(() => {
    if (snap.connectionError === "bluetoothUnavailable") return t.disconnected.bluetoothTitle;
    const parts = [t.connection[snap.connection]];
    if (snap.connection === "connected" && snap.status.batteryPct !== null) {
      parts.push(`${snap.status.batteryPct}%`);
    }
    return parts.join(" · ");
  });
</script>

<aside class="sidebar">
  <div class="titlebar" data-tauri-drag-region></div>

  <button
    type="button"
    class="device"
    onclick={() => prefs.go("devices")}
    aria-label="{deviceName}, {stateLine}"
  >
    <span class="glyph">
      <DeviceGlyph {finish} dim={tone !== "ok"} tone={slitTone} />
    </span>
    <span class="device-text">
      <span class="device-name">{deviceName}</span>
      <span class="device-state">
        <span class="dot" style:background={TONE_COLOR[tone]}></span>
        <span class="num state-text" title={stateLine}>{stateLine}</span>
      </span>
    </span>
  </button>

  <nav aria-label={t.nav.label} bind:this={nav}>
    <span
      class="pill"
      class:shown={pillShown}
      class:armed={pillArmed}
      style:transform="translateY({pillY}px)"
      aria-hidden="true"
    ></span>
    <ul>
      {#each primary as item (item.id)}
        <li>
          <button
            type="button"
            class="item"
            bind:this={items[item.id]}
            aria-current={prefs.section === item.id ? "page" : undefined}
            onclick={() => prefs.go(item.id)}
          >
            <Icon name={item.icon} size={16} />
            <span>{t.nav[item.id]}</span>
          </button>
        </li>
      {/each}
    </ul>
    <ul class="secondary">
      <li>
        <button
          type="button"
          class="item"
          bind:this={items.console}
          aria-current={prefs.section === "console" ? "page" : undefined}
          onclick={() => prefs.go("console")}
        >
          <Icon name="console" size={16} />
          <span>{t.nav.console}</span>
        </button>
      </li>
    </ul>
  </nav>

  <div class="language">
    <SegmentedControl
      size="small"
      label={t.nav.language}
      options={LOCALES.map((l) => ({ value: l, label: l === "en" ? "English" : "Русский" }))}
      value={i18n.locale}
      onchange={(l: Locale) => i18n.set(l)}
    />
  </div>
</aside>

<style>
  .sidebar {
    position: relative;
    display: flex;
    flex-direction: column;
    width: var(--sidebar-width);
    height: 100%;
    flex: none;
    background: var(--sidebar);
    border-right: 0.5px solid rgba(0, 0, 0, 0.55);
    box-shadow: inset -0.5px 0 0 rgba(255, 255, 255, 0.04);
  }
  .titlebar {
    height: 46px;
    flex: none;
  }

  .device {
    display: flex;
    align-items: center;
    gap: 11px;
    margin: 0 10px 14px;
    padding: 7px 8px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    text-align: left;
    transition:
      background-color var(--dur-fast) ease,
      transform var(--dur-fast) var(--ease-out);
  }
  .device:hover {
    background: var(--fill);
  }
  .device:active {
    transform: scale(0.985);
  }
  /* A quiet plinth for the little device: one flat tone and a hairline. */
  .glyph {
    display: block;
    width: 40px;
    height: 40px;
    padding: 4.5px 0;
    border-radius: 10px;
    background: var(--fill);
    box-shadow: inset 0 0 0 0.5px var(--hairline);
    flex: none;
  }
  .device-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .device-name {
    font: var(--text-headline);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .device-state {
    display: flex;
    align-items: center;
    gap: 6px;
    font: var(--text-callout);
    color: var(--label-2);
    white-space: nowrap;
    overflow: hidden;
  }
  .state-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex: none;
    transition: background-color var(--dur) ease;
  }

  nav {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 0 10px;
    overflow-y: auto;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .secondary {
    padding-top: 12px;
    border-top: 0.5px solid var(--hairline);
    margin: 0 8px;
  }
  .secondary .item {
    margin: 0 -8px;
    width: calc(100% + 16px);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    height: 30px;
    padding: 0 10px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--label);
    text-align: left;
  }
  .item :global(.icon) {
    color: var(--label-2);
  }
  .item {
    position: relative;
    transition:
      background-color var(--dur-fast) ease,
      transform var(--dur-fast) var(--ease-out);
  }
  .item :global(.icon) {
    transition: color var(--dur) ease;
  }
  .item:hover:not([aria-current="page"]) {
    background: var(--fill);
  }
  .item:active {
    transform: scale(0.985);
  }
  .pill {
    position: absolute;
    top: 0;
    left: 10px;
    right: 10px;
    height: 30px;
    border-radius: 7px;
    background: var(--selection);
    box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.04);
    opacity: 0;
    pointer-events: none;
  }
  .pill.shown {
    opacity: 1;
  }
  .pill.armed {
    transition: transform 340ms var(--ease-out);
  }
  .item[aria-current="page"] :global(.icon) {
    color: var(--label);
  }

  .language {
    padding: 12px 16px 16px;
  }
</style>
