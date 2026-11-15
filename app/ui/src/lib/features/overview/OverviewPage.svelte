<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Group from "$lib/components/Group.svelte";
  import Page from "$lib/components/Page.svelte";
  import ActivityList from "$lib/features/activity/ActivityList.svelte";
  import DeviceViewer from "$lib/features/device3d/DeviceViewer.svelte";
  import { detectedFinish, FINISH_SPECS } from "$lib/features/device3d/finishes";
  import DisconnectedState from "$lib/features/shell/DisconnectedState.svelte";
  import { i18n } from "$lib/i18n/index.svelte";
  import { activity } from "$lib/state/activity.svelte";
  import { prefs } from "$lib/state/prefs.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";
  import BatteryReadout from "./BatteryReadout.svelte";
  import CurvePanel from "./CurvePanel.svelte";
  import DeviceControls from "./DeviceControls.svelte";
  import Readings from "./Readings.svelte";

  const t = $derived(i18n.t);
  const snap = $derived(snapshot.current);
  const status = $derived(snap.status);

  let expanded = $state(false);
  let customModel = $state(false);
  const finish = $derived(prefs.finishFor(snap.device));
  const colorName = $derived.by(() => {
    const color = snap.device?.color;
    if (!color || customModel) return null;
    const known = detectedFinish(snap.device);
    return known ? t.device3d.finishes[known] : t.device3d.unknownColor(color.code);
  });

  // Remember the color of the device on screen, to keep it while disconnected.
  $effect(() => {
    if (snap.device) prefs.rememberFinish(finish);
  });
  const recent = $derived(expanded ? activity.events.slice(0, 40) : activity.events.slice(0, 6));

  const stateLine = $derived(
    [t.connection.connected, status.activity ? t.activity[status.activity] : null]
      .filter(Boolean)
      .join(" · "),
  );
  const versions = $derived(
    [
      snap.device?.firmware ? t.overview.firmware(snap.device.firmware) : null,
      snap.device?.hardware ? t.overview.hardware(snap.device.hardware) : null,
    ]
      .filter(Boolean)
      .join(" · "),
  );
</script>

<Page title={t.overview.title} wide>
  <section class="hero">
    <div class="stage">
      <DeviceViewer snapshot={snap} {finish} bind:custom={customModel} />
    </div>
    {#if snapshot.connected}
      <div class="identity">
        <div class="who">
          <h2 class="name selectable">{snap.device?.name ?? t.shell.noDevice}</h2>
          <p class="line">
            <span class="dot" aria-hidden="true"></span>
            {#key stateLine}<span class="state">{stateLine}</span>{/key}
          </p>
          {#if versions}<p class="versions num">{versions}</p>{/if}
          {#if colorName}
            <p class="finish-name">
              <span
                class="chip"
                style:background={FINISH_SPECS[finish].swatch[0]}
                aria-hidden="true"
              ></span>
              {colorName}
            </p>
          {/if}
        </div>
        <BatteryReadout />
      </div>
    {:else}
      <DisconnectedState hero />
    {/if}
  </section>

  {#if snapshot.connected}
    <CurvePanel />
    <Readings />

    <div class="columns">
      <DeviceControls />
      <Group title={t.overview.recent}>
        {#snippet actions()}
          {#if activity.events.length > 6}
            <Button variant="plain" size="small" onclick={() => (expanded = !expanded)}>
              {expanded ? t.overview.showLess : t.overview.showAll}
            </Button>
          {/if}
        {/snippet}
        <ActivityList events={recent} />
      </Group>
    </div>
  {/if}
</Page>

<style>
  /*
   * The device stands on the left, like a product shot; identity beside it and the charge dial
   * closing the row.
   */
  .hero {
    display: flex;
    align-items: center;
    gap: 26px;
    min-height: var(--stage-height);
    --stage-height: clamp(236px, 37vh, 300px);
  }
  .stage {
    flex: none;
    width: calc(var(--stage-height) * 0.66);
    height: var(--stage-height);
    /* Let the device's own margin, not the canvas edge, line up with the panels below. */
    margin: -8px -6px -10px -18px;
  }
  .identity {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 0 6px 0 0;
  }
  .who {
    min-width: 0;
  }
  .name {
    font: 600 30px/1.12 var(--font);
    letter-spacing: -0.022em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    font: 400 14px/1.3 var(--font);
    color: var(--label-2);
    white-space: nowrap;
  }
  .state {
    overflow: hidden;
    text-overflow: ellipsis;
    animation: settle var(--dur) var(--ease-out);
  }
  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--green);
  }
  .versions {
    margin-top: 5px;
    font: var(--text-callout);
    color: var(--label-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .finish-name {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 18px;
    font: var(--text-callout);
    color: var(--label-2);
  }
  .chip {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.28);
  }
  .columns {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 26px 20px;
    align-items: start;
  }
</style>
