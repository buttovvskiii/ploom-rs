<script lang="ts">
  interface Item {
    key: string;
    label: string;
    value: number;
    display: string;
  }
  interface Props {
    items: Item[];
    /** Full-scale value; defaults to the largest item. */
    max?: number;
  }
  let { items, max }: Props = $props();
  const scale = $derived(max ?? Math.max(1, ...items.map((i) => i.value)));
</script>

<ul class="bars">
  {#each items as item, i (item.key)}
    <li style:--i={i}>
      <span class="label" title={item.label}>{item.label}</span>
      <span class="track" aria-hidden="true">
        <span class="fill" style:width="{(item.value / scale) * 100}%"></span>
      </span>
      <span class="value num">{item.display}</span>
    </li>
  {/each}
</ul>

<style>
  .bars {
    list-style: none;
    margin: 0;
    padding: 8px 0;
  }
  li {
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(80px, 1fr) 5ch;
    align-items: center;
    gap: 14px;
    min-height: 30px;
    padding: 0 var(--row-x);
  }
  .label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .track {
    height: 4px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.06);
    overflow: hidden;
  }
  .fill {
    display: block;
    height: 100%;
    min-width: 2px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.55);
    /* Bars draw out one after another when the list appears. */
    transform-origin: left center;
    animation: grow var(--dur-slow) var(--ease-out) backwards;
    animation-delay: calc(var(--i) * 45ms + 120ms);
    transition: width var(--dur-slow) var(--ease-out);
  }
  @keyframes grow {
    from {
      transform: scaleX(0);
    }
  }
  .value {
    text-align: right;
    color: var(--label-2);
  }
</style>
