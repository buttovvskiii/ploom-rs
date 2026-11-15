<script lang="ts">
  interface Props {
    rssi: number | null;
    label?: string;
    size?: "small" | "regular";
  }
  let { rssi, label, size = "regular" }: Props = $props();

  const level = $derived(
    rssi === null ? 0 : rssi >= -60 ? 4 : rssi >= -70 ? 3 : rssi >= -80 ? 2 : rssi >= -92 ? 1 : 0,
  );
  const heights = [4, 7, 10, 13];
</script>

<svg
  class="bars {size}"
  viewBox="0 0 18 14"
  role={label ? "img" : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : "true"}
>
  {#each heights as h, i (i)}
    <rect
      x={i * 4.6}
      y={14 - h}
      width="3"
      height={h}
      rx="1"
      fill="currentColor"
      style:opacity={i < level ? 0.92 : 0.18}
    />
  {/each}
</svg>

<style>
  .bars {
    flex: none;
    display: block;
  }
  rect {
    transition: opacity var(--dur) var(--ease-out);
  }
  .regular {
    width: 18px;
    height: 14px;
  }
  .small {
    width: 14px;
    height: 11px;
  }
</style>
