<script lang="ts">
  interface Props {
    durationS: number;
    offsetsS: number[];
    label: string;
    minutesUnit: string;
  }
  let { durationS, offsetsS, label, minutesUnit }: Props = $props();

  let width = $state(0);
  const height = 62;
  const pad = { left: 1, right: 1, top: 8, bottom: 20 };
  const total = $derived(Math.max(1, durationS));
  const x = (t: number) =>
    pad.left + (Math.min(Math.max(t, 0), total) / total) * (width - pad.left - pad.right);
  const baseY = height - pad.bottom;

  const ticks = $derived.by(() => {
    const out: number[] = [];
    for (let m = 0; m * 60 <= total; m++) out.push(m);
    return out;
  });
</script>

<div class="timeline" bind:clientWidth={width}>
  {#if width > 0}
    <svg {width} {height} role="img" aria-label={label}>
      <rect class="span" x={pad.left} y={baseY - 1} width={x(total) - pad.left} height="2" rx="1" />
      {#each ticks as m (m)}
        <line class="tick" x1={x(m * 60)} x2={x(m * 60)} y1={baseY + 3} y2={baseY + 6} />
        <text class="axis" x={x(m * 60)} y={height - 3} text-anchor={m === 0 ? "start" : "middle"}>
          {m === ticks.at(-1) ? `${m} ${minutesUnit}` : m}
        </text>
      {/each}
      {#each offsetsS as t, i (i)}
        <line
          class="puff"
          x1={x(t)}
          x2={x(t)}
          y1={pad.top}
          y2={baseY - 4}
          style:animation-delay="{120 + i * 28}ms"
        />
      {/each}
    </svg>
  {/if}
</div>

<style>
  .timeline {
    width: 100%;
    height: 62px;
  }
  svg {
    display: block;
    overflow: visible;
  }
  .span {
    fill: rgba(255, 255, 255, 0.14);
  }
  .tick {
    stroke: var(--label-4);
    stroke-width: 1;
  }
  .axis {
    fill: var(--label-3);
    font: 400 10px/1 var(--font);
    font-variant-numeric: tabular-nums;
  }
  .puff {
    stroke: var(--label);
    stroke-width: 2;
    stroke-linecap: round;
    opacity: 0.85;
    /* Puffs appear in the order they were taken. */
    animation: puff var(--dur) var(--ease-out) backwards;
  }
  @keyframes puff {
    from {
      opacity: 0;
      transform: translateY(5px);
    }
  }
</style>
