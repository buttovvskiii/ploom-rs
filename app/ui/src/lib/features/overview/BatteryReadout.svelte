<script lang="ts" module>
  // The opening sweep plays once per run, not every time Overview is shown.
  let introduced = false;
</script>

<script lang="ts">
  import { untrack } from "svelte";
  import { Tween } from "svelte/motion";
  import Icon from "$lib/components/icons/Icon.svelte";
  import { i18n } from "$lib/i18n/index.svelte";
  import { easeOut, reducedMotion } from "$lib/motion";
  import { snapshot } from "$lib/state/snapshot.svelte";

  const t = $derived(i18n.t);
  const status = $derived(snapshot.status);
  const charging = $derived(status.charging === true);
  const pct = $derived(status.batteryPct);

  // A 270° dial of fine ticks, open at the bottom; lit ticks show the charge.
  const TICKS = 48;
  const SIZE = 136;
  const C = SIZE / 2;
  const R_IN = 53;
  const R_OUT = 62;
  const START = 135;
  const SWEEP = 270;
  const ticks = Array.from({ length: TICKS }, (_, i) => {
    const a = ((START + (SWEEP * i) / (TICKS - 1)) * Math.PI) / 180;
    const cos = Math.cos(a);
    const sin = Math.sin(a);
    return { x1: C + R_IN * cos, y1: C + R_IN * sin, x2: C + R_OUT * cos, y2: C + R_OUT * sin };
  });

  // The dial sweeps up to the first reading and glides between later ones; the figure counts along
  // with it.
  const level = new Tween(
    untrack(() => (introduced ? (pct ?? 0) : 0)),
    { duration: 0, easing: easeOut },
  );
  $effect(() => {
    if (pct === null) return;
    const duration = reducedMotion() ? 0 : introduced ? 600 : 1100;
    introduced = true;
    void level.set(pct, { duration });
  });

  const lit = $derived(pct === null ? 0 : Math.round((level.current / 100) * TICKS));
  const shown = $derived(pct === null ? "—" : String(Math.round(level.current)));
  const tone = $derived(
    charging ? "var(--green)" : pct !== null && pct <= 15 ? "var(--red)" : "var(--accent)",
  );
  const mode = $derived(charging ? "charging" : status.usbPlugged ? "plugged" : "charge");
</script>

<div
  class="battery"
  class:charging
  role="group"
  aria-label={t.overview.battery}
  style:--tone={tone}
>
  <svg width={SIZE} height={SIZE} viewBox="0 0 {SIZE} {SIZE}" aria-hidden="true">
    {#each ticks as tick, i (i)}
      <line
        {...tick}
        class:lit={i < lit}
        class:next={i === lit && lit < TICKS}
        style:opacity={i < lit ? 0.42 + (0.58 * (i + 1)) / Math.max(1, lit) : undefined}
      />
    {/each}
  </svg>
  <div class="center">
    <div class="figure num" aria-label={pct === null ? t.common.notReported : `${pct}%`}>
      <span class="value">{shown}</span><span class="unit">%</span>
    </div>
    {#key mode}
      <div class="caption">
        {#if mode === "charging"}
          <Icon name="bolt" size={11} />
        {:else if mode === "plugged"}
          <Icon name="plug" size={11} />
        {/if}
        <span>{t.overview.dial[mode]}</span>
      </div>
    {/key}
  </div>
</div>

<style>
  .battery {
    position: relative;
    flex: none;
    width: 136px;
    height: 136px;
  }
  svg {
    display: block;
  }
  line {
    stroke: rgba(255, 255, 255, 0.09);
    stroke-width: 2;
    stroke-linecap: round;
    transition:
      stroke var(--dur) var(--ease-out),
      opacity var(--dur) var(--ease-out);
  }
  .lit {
    stroke: var(--tone);
  }
  /* While charging, the tick being filled breathes: the dial visibly takes charge. */
  .charging .next {
    stroke: var(--tone);
    animation: fill 1.8s var(--ease-in-out) infinite;
  }
  @keyframes fill {
    0%,
    100% {
      opacity: 0.12;
    }
    50% {
      opacity: 0.85;
    }
  }
  .center {
    position: absolute;
    /* Stay inside the ring: the caption can never reach the ticks. */
    inset: 22px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 5px;
  }
  .figure {
    display: flex;
    align-items: baseline;
    line-height: 1;
  }
  .value {
    font: 500 34px/1 var(--font);
    letter-spacing: -0.025em;
  }
  .unit {
    margin-left: 2px;
    font: 400 14px/1 var(--font);
    color: var(--label-2);
  }
  .caption {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    max-width: 100%;
    font: var(--text-footnote);
    color: var(--label-2);
    white-space: nowrap;
    animation: settle var(--dur) var(--ease-out);
  }
  .caption span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .caption :global(.icon) {
    flex: none;
    color: var(--tone);
  }
</style>
