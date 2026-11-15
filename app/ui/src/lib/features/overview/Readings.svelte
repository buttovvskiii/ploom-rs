<script lang="ts">
  import AnimatedNumber from "$lib/components/AnimatedNumber.svelte";
  import SignalBars from "$lib/components/SignalBars.svelte";
  import { formatTemp } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/index.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";

  const t = $derived(i18n.t);
  const status = $derived(snapshot.status);
</script>

<dl class="readings">
  <div class="cell">
    <dt>{t.overview.signal}</dt>
    <dd>
      <SignalBars rssi={status.rssiDbm} />
      <AnimatedNumber value={status.rssiDbm} format={(v) => t.devices.signal(v)} />
    </dd>
  </div>
  <div class="cell">
    <dt>{t.overview.enclosure}</dt>
    <dd><AnimatedNumber value={status.enclosureTempC} digits={1} format={formatTemp} /></dd>
  </div>
  <div class="cell">
    <dt>{t.overview.health}</dt>
    <dd><AnimatedNumber value={status.batteryHealthPct} format={(v) => `${v}%`} /></dd>
  </div>
</dl>

<style>
  .readings {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    margin: 0;
    border-radius: var(--radius-group);
    background: var(--group);
    box-shadow: var(--surface-edge);
  }
  .cell {
    position: relative;
    padding: 12px 16px 13px;
    min-width: 0;
  }
  .cell + .cell::before {
    content: "";
    position: absolute;
    left: 0;
    top: 12px;
    bottom: 12px;
    width: 0.5px;
    background: var(--hairline-strong);
  }
  dt {
    font: var(--text-callout);
    color: var(--label-2);
  }
  dd {
    margin: 4px 0 0;
    display: flex;
    align-items: center;
    gap: 8px;
    font: 500 17px/1.2 var(--font);
    white-space: nowrap;
  }
  dd :global(.bars) {
    color: var(--label);
  }
</style>
