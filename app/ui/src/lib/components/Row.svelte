<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    label?: string;
    detail?: string | null;
    /** Leading content (icon, thumbnail). */
    leading?: Snippet;
    /** Replaces the label block entirely. */
    content?: Snippet;
    children?: Snippet;
    /** Stack trailing content under the label (for sliders, wide controls). */
    stacked?: boolean;
  }

  let { label, detail, leading, content, children, stacked = false }: Props = $props();
</script>

<div class="row" class:stacked>
  {#if leading}<div class="leading">{@render leading()}</div>{/if}
  <div class="text">
    {#if content}
      {@render content()}
    {:else}
      <span class="label">{label}</span>
      {#if detail}<span class="detail">{detail}</span>{/if}
    {/if}
  </div>
  {#if children}<div class="trailing">{@render children()}</div>{/if}
</div>

<style>
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 8px var(--row-x);
  }
  .row:not(:first-child)::before {
    content: "";
    position: absolute;
    top: 0;
    left: var(--row-x);
    right: 0;
    height: 0.5px;
    background: var(--hairline-strong);
  }
  .stacked {
    flex-wrap: wrap;
  }
  .stacked .trailing {
    flex-basis: 100%;
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
    margin-top: 2px;
  }
  .leading {
    flex: none;
    display: flex;
    align-items: center;
    color: var(--label-2);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .label {
    color: var(--label);
  }
  .detail {
    font: var(--text-callout);
    color: var(--label-2);
  }
  .trailing {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--label-2);
  }
</style>
