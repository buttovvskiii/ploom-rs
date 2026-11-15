<script lang="ts">
  import { GLYPHS, type Glyph, type IconName } from "./paths";

  interface Props {
    name: IconName;
    size?: number;
    /** Accessible name; omit for decorative icons next to visible text. */
    label?: string;
    strokeWidth?: number;
  }

  let { name, size = 16, label, strokeWidth = 1.5 }: Props = $props();
  const glyph: Glyph = $derived(GLYPHS[name]);
</script>

<svg
  class="icon"
  width={size}
  height={size}
  viewBox="0 0 20 20"
  role={label ? "img" : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : "true"}
>
  {#each glyph.stroke as d (d)}
    <path
      {d}
      fill="none"
      stroke="currentColor"
      stroke-width={strokeWidth}
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  {/each}
  {#each glyph.fill ?? [] as d (d)}
    <path {d} fill="currentColor" />
  {/each}
</svg>

<style>
  .icon {
    flex: none;
    display: block;
  }
</style>
