<script lang="ts">
  interface Props {
    used: number;
    total: number;
    label: string;
    size?: "regular" | "small";
    locked?: boolean;
  }
  let { used, total, label, size = "regular", locked = false }: Props = $props();
  const marks = $derived(Array.from({ length: total }, (_, i) => i < used));
</script>

<div class="marks {size}" class:locked class:dense={total > 20} role="img" aria-label={label}>
  {#each marks as on, i (i)}
    <span class="mark" class:on></span>
  {/each}
</div>

<style>
  .marks {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    --w: 7px;
    --h: 26px;
  }
  .marks.dense {
    gap: 4px;
    --w: 5px;
  }
  .marks.small {
    gap: 3px;
    flex-wrap: nowrap;
    --w: 3px;
    --h: 12px;
  }
  .mark {
    width: var(--w);
    height: var(--h);
    border-radius: calc(var(--w) / 2);
    background: var(--fill-strong);
    transition: background-color var(--dur) var(--ease-out);
  }
  .mark.on {
    background: var(--accent);
  }
  .locked .mark.on {
    background: var(--label-2);
  }
</style>
