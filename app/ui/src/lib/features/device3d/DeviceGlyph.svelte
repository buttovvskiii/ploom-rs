<script lang="ts">
  /**
   * The device seen from the front, drawn as a vector in its real finish: the shouldered body, the
   * low notch in the cap, the cap seam, the glossy front panel and the light slit in its upper
   * third.
   */
  import { FINISH_SPECS, type FinishId } from "./finishes";

  interface Props {
    finish: FinishId;
    /** Dimmed and desaturated when the device is not connected. */
    dim?: boolean;
    /** Light-slit color (state tone); omitted leaves the slit unlit. */
    tone?: string | null;
    /** Draw fine details at true scale; off keeps the slit legible when small. */
    detailed?: boolean;
  }
  let { finish, dim = false, tone = null, detailed = false }: Props = $props();

  const id = $props.id();
  const spec = $derived(FINISH_SPECS[finish]);

  // Two drawings from one recipe.
  const H = 110;
  const g = $derived.by(() => {
    const W = detailed ? 26 : 40;
    const x0 = 1;
    const x1 = W - 1;
    const rt = detailed ? 8 : 11; // shoulders
    const rb = ((W - 2) / 2) * (detailed ? 0.9 : 0.8); // base
    const body =
      `M${x0 + rt} 1H${x1 - rt}A${rt} ${rt} 0 0 1 ${x1} ${1 + rt}V${H - 1 - rb}` +
      `A${rb} ${rb} 0 0 1 ${x1 - rb} ${H - 1}H${x0 + rb}A${rb} ${rb} 0 0 1 ${x0} ${H - 1 - rb}` +
      `V${1 + rt}A${rt} ${rt} 0 0 1 ${x0 + rt} 1Z`;

    const seamY = detailed ? 9.4 : 12;
    const inset = detailed ? 2.2 : 3.6;
    const px0 = x0 + inset;
    const px1 = x1 - inset;
    const pTop = seamY + (detailed ? 1.4 : 2.4);
    const pBottom = H - 1 - (detailed ? 2.6 : 4.2);
    const pr = 3;
    const pb = Math.max(4, rb - inset * 0.9);
    const panel =
      `M${px0 + pr} ${pTop}H${px1 - pr}A${pr} ${pr} 0 0 1 ${px1} ${pTop + pr}V${pBottom - pb}` +
      `A${pb} ${pb} 0 0 1 ${px1 - pb} ${pBottom}H${px0 + pb}A${pb} ${pb} 0 0 1 ${px0} ${pBottom - pb}` +
      `V${pTop + pr}A${pr} ${pr} 0 0 1 ${px0 + pr} ${pTop}Z`;

    const nw = W * (detailed ? 0.5 : 0.46);
    const nd = detailed ? 6.4 : 8.4;
    const nr = detailed ? 3 : 3.6;
    const nx = (W - nw) / 2;
    const notch =
      `M${nx} 0H${nx + nw}V${nd - nr}A${nr} ${nr} 0 0 1 ${nx + nw - nr} ${nd}` +
      `H${nx + nr}A${nr} ${nr} 0 0 1 ${nx} ${nd - nr}Z`;

    const sw = detailed ? 1.7 : 4.4;
    return {
      W,
      body,
      panel,
      notch,
      seam: `M${x0} ${seamY}H${x1}`,
      seamWidth: detailed ? 0.8 : 1.5,
      slit: { x: (W - sw) / 2, y: detailed ? 18 : 22, w: sw, h: detailed ? 12.6 : 17 },
    };
  });
</script>

<svg
  class="glyph"
  class:dim
  viewBox="0 0 {g.W} {H}"
  preserveAspectRatio="xMidYMid meet"
  aria-hidden="true"
>
  <defs>
    <linearGradient id="{id}-body" x1="0" x2="1" y1="0" y2="0">
      <stop offset="0" stop-color={spec.swatch[1]} />
      <stop offset="0.36" stop-color={spec.swatch[0]} />
      <stop offset="1" stop-color={spec.swatch[1]} />
    </linearGradient>
    <linearGradient id="{id}-gloss" x1="0" x2="1" y1="0" y2="0.12">
      <stop offset="0" stop-color="#fff" stop-opacity="0.3" />
      <stop offset="0.45" stop-color="#fff" stop-opacity="0.03" />
      <stop offset="1" stop-color="#000" stop-opacity="0.16" />
    </linearGradient>
    <clipPath id="{id}-clip"><path d={g.body} /></clipPath>
  </defs>

  <g clip-path="url(#{id}-clip)">
    <rect width={g.W} height={H} fill="url(#{id}-body)" />
    <path d={g.notch} class="notch" />
    <path d={g.seam} class="seam" stroke-width={g.seamWidth} />
    <path d={g.panel} fill={spec.swatchCover} />
    <path d={g.panel} fill="url(#{id}-gloss)" class="panel-edge" />
  </g>
  <rect
    x={g.slit.x}
    y={g.slit.y}
    width={g.slit.w}
    height={g.slit.h}
    rx={g.slit.w / 2}
    class="slit"
    class:lit={tone !== null}
    style:fill={tone}
  />
  <path d={g.body} class="rim" />
</svg>

<style>
  .glyph {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
    transition:
      opacity var(--dur-slow) var(--ease-out),
      filter var(--dur-slow) var(--ease-out);
  }
  .notch {
    fill: rgba(0, 0, 0, 0.5);
  }
  .seam {
    fill: none;
    stroke: rgba(0, 0, 0, 0.45);
  }
  .panel-edge {
    stroke: rgba(0, 0, 0, 0.3);
    stroke-width: 0.7;
  }
  .slit {
    fill: rgba(127, 127, 127, 0.32);
    /* A hairline keeps a white slit visible on the light finishes. */
    stroke: rgba(0, 0, 0, 0.22);
    stroke-width: 0.5;
    transition: fill var(--dur-slow) var(--ease-out);
  }
  .rim {
    fill: none;
    stroke: rgba(255, 255, 255, 0.2);
    vector-effect: non-scaling-stroke;
    stroke-width: 0.5px;
  }
  .dim {
    opacity: 0.6;
    filter: saturate(0.4);
  }
</style>
