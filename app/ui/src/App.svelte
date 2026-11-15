<script lang="ts">
  import { onMount, type Component } from "svelte";
  import Toaster from "$lib/components/Toaster.svelte";
  import ConsolePage from "$lib/features/console/ConsolePage.svelte";
  import DevicesPage from "$lib/features/devices/DevicesPage.svelte";
  import DiagnosticsPage from "$lib/features/diagnostics/DiagnosticsPage.svelte";
  import HeatingPage from "$lib/features/heating/HeatingPage.svelte";
  import ConnectGate from "$lib/features/gate/ConnectGate.svelte";
  import HistoryPage from "$lib/features/history/HistoryPage.svelte";
  import LimitPage from "$lib/features/limit/LimitPage.svelte";
  import OverviewPage from "$lib/features/overview/OverviewPage.svelte";
  import Sidebar from "$lib/features/shell/Sidebar.svelte";
  import { fade, shift } from "$lib/motion";
  import { isTauri } from "$lib/api";
  import { i18n } from "$lib/i18n/index.svelte";
  import { backend, start } from "$lib/state/api.svelte";
  import { prefs, type Section } from "$lib/state/prefs.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";

  const PAGES: Record<Section, Component> = {
    overview: OverviewPage,
    heating: HeatingPage,
    limit: LimitPage,
    history: HistoryPage,
    diagnostics: DiagnosticsPage,
    devices: DevicesPage,
    console: ConsolePage,
  };
  const ORDER: Section[] = [
    "overview",
    "heating",
    "limit",
    "history",
    "diagnostics",
    "devices",
    "console",
  ];
  // Nothing but the connect screen until a device is connected.
  const gated = $derived(snapshot.current.device === null);

  const Current = $derived(PAGES[prefs.section]);

  onMount(() => {
    if (isTauri) void start();
  });

  function onkeydown(event: KeyboardEvent): void {
    // ⌘1…⌘7 switch sections, like the tabs of a macOS settings window.
    if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey) {
      const section = ORDER[Number(event.key) - 1];
      if (section && !gated) {
        event.preventDefault();
        prefs.go(section);
      }
    }
  }
</script>

<svelte:window {onkeydown} />

{#if !isTauri}
  <div class="window">
    <div class="failure">
      <p>{i18n.t.shell.desktopOnly}</p>
    </div>
  </div>
{:else if !backend.ready}
  <div class="window">
    {#if backend.failure}
      <div class="failure" role="alert">
        <p>{backend.failure}</p>
      </div>
    {:else}
      <div class="titlebar" data-tauri-drag-region></div>
    {/if}
  </div>
{:else if gated}
  <div class="layer" out:fade={{ duration: 200 }} in:fade={{ duration: 260 }}>
    <ConnectGate />
  </div>
{:else}
  <div class="layer window" in:shift={{ duration: 460, delay: 120, scale: 0.985 }} out:fade>
    <Sidebar />
    <main class="main">
      {#key prefs.section}
        <div class="view" in:fade={{ duration: 220, delay: 70 }} out:fade={{ duration: 110 }}>
          <Current />
        </div>
      {/key}
    </main>
  </div>
{/if}

<Toaster />

<style>
  .window {
    display: flex;
    height: 100%;
    min-width: 0;
  }
  .layer {
    position: absolute;
    inset: 0;
  }
  .main {
    position: relative;
    flex: 1;
    min-width: 0;
    background: var(--window);
  }
  .view {
    position: absolute;
    inset: 0;
  }
  .titlebar {
    height: var(--titlebar);
  }
  .failure {
    display: grid;
    place-items: center;
    flex: 1;
    height: 100%;
    padding: 40px;
    color: var(--label-2);
    text-align: center;
  }
</style>
