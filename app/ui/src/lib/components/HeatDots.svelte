<script lang="ts">
  interface Props {
    heat: 1 | 2 | 3;
    tint: string;
    label: string;
  }
  let { heat, tint, label }: Props = $props();
</script>

<span class="dots" role="img" aria-label={label}>
  {#each [1, 2, 3] as i (i)}
    <span class="dot" style:background={i <= heat ? tint : undefined}></span>
  {/each}
</span>

<style>
  .dots {
    display: inline-flex;
    gap: 3px;
  }
  .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.14);
  }
</style>
