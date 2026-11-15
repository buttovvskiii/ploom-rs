<script lang="ts">
  import type { CurveStep } from "$lib/api";
  import { curvePoints, levelAt, smoothPath, totalDuration, yAtX } from "$lib/curve/curve";

  interface Props {
    curve: readonly CurveStep[];
    tint: string;
    /** Temperature range of the plot; share one across plots so they compare. */
    domain: [number, number];
    label: string;
    variant?: "full" | "thumb";
    height?: number;
    /** Seconds into the session; draws the playhead. */
    elapsedS?: number | null;
    /** [start, end] seconds of a phase to emphasize (the custom hold step). */
    highlight?: [number, number] | null;
    /** Minutes label for the time axis ("min"/"мин"). */
    minutesUnit?: string;
    /** A reference curve drawn faintly behind (the unmodified preset). */
    ghost?: readonly CurveStep[] | null;
    /** Draw the line in when the curve changes (not for live-edited curves). */
    animate?: boolean;
  }

  let {
    curve,
    tint,
    domain,
    label,
    variant = "full",
    height = 200,
    elapsedS = null,
    highlight = null,
    minutesUnit = "min",
    ghost = null,
    animate = false,
  }: Props = $props();

  const uid = $props.id();
  let measured = $state(0);

  const full = $derived(variant === "full");
  const width = $derived(full ? measured : 64);
  const pad = $derived(
    full ? { top: 22, right: 42, bottom: 24, left: 1 } : { top: 3, right: 1, bottom: 3, left: 1 },
  );
  const plotW = $derived(Math.max(0, width - pad.left - pad.right));
  const plotH = $derived(Math.max(0, height - pad.top - pad.bottom));

  const total = $derived(Math.max(1, totalDuration(curve), ghost ? totalDuration(ghost) : 0));
  const points = $derived(curvePoints(curve));

  const x = (t: number) => pad.left + (Math.min(Math.max(t, 0), total) / total) * plotW;
  const y = (c: number) => {
    const [lo, hi] = domain;
    const v = c <= 0 ? lo : Math.min(Math.max(c, lo), hi);
    return pad.top + plotH - ((v - lo) / (hi - lo)) * plotH;
  };

  // Corners are rounded generously in the full plot and barely in thumbnails.
  const radius = $derived(full ? 60 : 6);
  const ease = $derived(full ? 22 : 2);
  const shape = $derived(
    smoothPath(
      points.map((p) => ({ x: x(p.t), y: y(p.c) })),
      radius,
      ease,
    ),
  );
  const path = $derived(shape.d);
  const area = $derived(
    `${path}L${x(points.at(-1)?.t ?? 0).toFixed(2)} ${y(0).toFixed(2)}L${x(0).toFixed(2)} ${y(0).toFixed(2)}Z`,
  );

  const ghostPath = $derived(
    ghost
      ? smoothPath(
          curvePoints(ghost).map((p) => ({ x: x(p.t), y: y(p.c) })),
          radius,
          ease,
        ).d
      : null,
  );

  const gridTemps = $derived.by(() => {
    const [lo, hi] = domain;
    const out: number[] = [];
    for (let v = Math.ceil((lo + 1) / 50) * 50; v < hi; v += 50) out.push(v);
    return out;
  });

  const minuteTicks = $derived.by(() => {
    const minutes = Math.floor(total / 60);
    const every = minutes > 9 ? 2 : 1;
    const out: number[] = [];
    for (let m = 0; m <= minutes; m += every) out.push(m);
    return out;
  });

  const playhead = $derived.by(() => {
    if (elapsedS === null) return null;
    const t = Math.min(Math.max(elapsedS, 0), total);
    const px = x(t);
    return { x: px, y: yAtX(shape.samples, px), tempC: Math.round(levelAt(points, t)) };
  });
  /** The target-temperature pill above the playhead, kept inside the plot. */
  const pill = $derived.by(() => {
    if (!playhead || playhead.tempC <= 0) return null;
    const text = `${playhead.tempC}°`;
    const w = 12 + text.length * 6.2;
    const cx = Math.min(Math.max(playhead.x, pad.left + w / 2), pad.left + plotW - w / 2);
    return { text, w, cx, y: Math.max(2, playhead.y - 30) };
  });
</script>

<div class="plot {variant}" bind:clientWidth={measured} style:height="{height}px">
  {#if width > 0}
    <svg {width} {height} role="img" aria-label={label}>
      <defs>
        <clipPath id="{uid}-past">
          <rect x="0" y="0" width={playhead ? playhead.x : width} {height} />
        </clipPath>
        {#if full}
          <linearGradient
            id="{uid}-fill"
            x1="0"
            y1={pad.top}
            x2="0"
            y2={y(0)}
            gradientUnits="userSpaceOnUse"
          >
            <stop offset="0" style:stop-color={tint} stop-opacity="0.32" />
            <stop offset="1" style:stop-color={tint} stop-opacity="0" />
          </linearGradient>
        {/if}
      </defs>

      {#if full}
        {#each gridTemps as v (v)}
          <line class="grid" x1={pad.left} x2={pad.left + plotW} y1={y(v)} y2={y(v)} />
          <text class="axis" x={width - 2} y={y(v) + 3.5} text-anchor="end">{v}°</text>
        {/each}
        <line class="baseline" x1={pad.left} x2={pad.left + plotW} y1={y(0)} y2={y(0)} />
        {#each minuteTicks as m (m)}
          <line class="tick" x1={x(m * 60)} x2={x(m * 60)} y1={y(0)} y2={y(0) + 4} />
          <text
            class="axis"
            x={x(m * 60)}
            y={height - 6}
            text-anchor={m === 0 ? "start" : "middle"}
          >
            {m === minuteTicks.at(-1) ? `${m} ${minutesUnit}` : m}
          </text>
        {/each}
      {:else}
        <line class="baseline thumb-base" x1={pad.left} x2={pad.left + plotW} y1={y(0)} y2={y(0)} />
      {/if}

      {#if highlight}
        <rect
          class="band"
          x={x(highlight[0])}
          y={pad.top}
          width={Math.max(0, x(highlight[1]) - x(highlight[0]))}
          height={plotH}
          style:fill={tint}
        />
        <line
          class="band-edge"
          x1={x(highlight[0])}
          x2={x(highlight[0])}
          y1={pad.top}
          y2={y(0)}
          style:stroke={tint}
        />
        <line
          class="band-edge"
          x1={x(highlight[1])}
          x2={x(highlight[1])}
          y1={pad.top}
          y2={y(0)}
          style:stroke={tint}
        />
      {/if}

      {#if ghostPath}
        <path class="ghost" d={ghostPath} />
      {/if}

      {#key animate ? curve : null}
        <g class:draw={animate}>
          {#if full}
            <path class="area" d={area} fill="url(#{uid}-fill)" />
            {#if playhead}
              <path
                class="area past"
                d={area}
                fill="url(#{uid}-fill)"
                clip-path="url(#{uid}-past)"
              />
            {/if}
            <path
              class="glow"
              d={path}
              pathLength="1"
              style:stroke={tint}
              clip-path={playhead ? `url(#${uid}-past)` : undefined}
            />
          {/if}

          {#if playhead}
            <path class="line future" d={path} pathLength="1" style:stroke={tint} />
          {/if}
          <path
            class="line"
            class:thin={!full}
            d={path}
            pathLength="1"
            style:stroke={tint}
            clip-path={playhead ? `url(#${uid}-past)` : undefined}
          />
        </g>
      {/key}

      {#if playhead && full}
        <line class="playhead" x1={playhead.x} x2={playhead.x} y1={playhead.y} y2={y(0)} />
        <circle class="pulse" cx={playhead.x} cy={playhead.y} r="5" style:fill={tint} />
        <circle class="head-ring" cx={playhead.x} cy={playhead.y} r="6.5" />
        <circle cx={playhead.x} cy={playhead.y} r="4" style:fill={tint} />
        {#if pill}
          <g class="pill">
            <rect x={pill.cx - pill.w / 2} y={pill.y} width={pill.w} height="19" rx="9.5" />
            <text x={pill.cx} y={pill.y + 13.2} text-anchor="middle">{pill.text}</text>
          </g>
        {/if}
      {/if}
    </svg>
  {/if}
</div>

<style>
  .plot {
    position: relative;
    width: 100%;
  }
  .plot.thumb {
    width: 64px;
    flex: none;
  }
  svg {
    display: block;
    overflow: visible;
  }
  .grid {
    stroke: var(--hairline);
    stroke-width: 1;
    shape-rendering: crispEdges;
  }
  .baseline {
    stroke: var(--hairline-strong);
    stroke-width: 1;
    shape-rendering: crispEdges;
  }
  .thumb-base {
    stroke: rgba(255, 255, 255, 0.1);
  }
  .tick {
    stroke: var(--label-4);
    stroke-width: 1;
    shape-rendering: crispEdges;
  }
  .axis {
    fill: var(--label-3);
    font: 400 10px/1 var(--font);
    font-variant-numeric: tabular-nums;
  }
  .line {
    fill: none;
    stroke-width: 2.2;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .line.thin {
    stroke-width: 1.4;
  }
  .future {
    opacity: 0.3;
  }
  .glow {
    fill: none;
    stroke-width: 9;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0.14;
  }
  .area {
    opacity: 0.35;
  }
  .area.past {
    opacity: 1;
  }
  .ghost {
    fill: none;
    stroke: var(--label-3);
    stroke-width: 1;
    stroke-dasharray: 3 3;
  }

  /* Draw-in: the line traces itself, the glow and fill follow. */
  .draw .line:not(.future),
  .draw .glow {
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    animation: trace 1100ms var(--ease-out) 80ms forwards;
  }
  .draw .area,
  .draw .future {
    animation: rise 900ms var(--ease-out) 350ms backwards;
  }
  @keyframes trace {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
    }
  }
  .band {
    opacity: 0.05;
  }
  .band-edge {
    stroke-width: 1;
    stroke-dasharray: 2 3;
    opacity: 0.45;
  }
  .playhead {
    stroke: var(--label-3);
    stroke-width: 1;
    stroke-dasharray: 2 3;
  }
  .head-ring {
    fill: var(--group);
  }
  .pulse {
    transform-box: fill-box;
    transform-origin: center;
    animation: pulse 2s var(--ease-out) infinite;
  }
  @keyframes pulse {
    from {
      transform: scale(1);
      opacity: 0.55;
    }
    to {
      transform: scale(3.2);
      opacity: 0;
    }
  }
  .pill rect {
    fill: var(--control);
    stroke: var(--hairline-strong);
    stroke-width: 0.5;
  }
  .pill text {
    fill: var(--label);
    font: 600 11px/1 var(--font);
    font-variant-numeric: tabular-nums;
  }
</style>
