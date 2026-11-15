<script lang="ts">
  interface Props {
    size?: number;
    label?: string;
  }
  let { size = 14, label }: Props = $props();
  const spokes = Array.from({ length: 8 }, (_, i) => i);
</script>

<svg
  class="spinner"
  width={size}
  height={size}
  viewBox="0 0 20 20"
  role={label ? "progressbar" : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : "true"}
>
  {#each spokes as i (i)}
    <line
      x1="10"
      y1="2.6"
      x2="10"
      y2="6.2"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      opacity={0.18 + (i / 8) * 0.82}
      transform="rotate({i * 45} 10 10)"
    />
  {/each}
</svg>

<style>
  .spinner {
    flex: none;
    animation: turn 0.8s steps(8) infinite;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }
</style>
