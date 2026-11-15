<script lang="ts">
  interface Props {
    value: number;
    max?: number;
    label: string;
    tint?: string;
  }
  let { value, max = 1, label, tint = "var(--accent-strong)" }: Props = $props();
  const fraction = $derived(max > 0 ? Math.min(1, Math.max(0, value / max)) : 0);
</script>

<div
  class="progress"
  role="progressbar"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={max}
  aria-valuenow={value}
>
  <div class="fill" style:transform="scaleX({fraction})" style:background={tint}></div>
</div>

<style>
  .progress {
    height: 4px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.1);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    transform-origin: left center;
    transition: transform 220ms linear;
  }
</style>
