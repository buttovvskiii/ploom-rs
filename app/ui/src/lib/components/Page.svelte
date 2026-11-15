<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    actions?: Snippet;
    children: Snippet;
    /** Wider content for pages that lay out in columns. */
    wide?: boolean;
  }
  let { title, actions, children, wide = false }: Props = $props();
</script>

<div class="page" class:wide>
  <header class="toolbar" data-tauri-drag-region>
    <div class="bar" data-tauri-drag-region>
      <h1 data-tauri-drag-region>{title}</h1>
      {#if actions}<div class="actions">{@render actions()}</div>{/if}
    </div>
  </header>
  <div class="scroll" onscroll={() => window.dispatchEvent(new Event("ploom:scroll"))}>
    <div class="content">
      {@render children()}
    </div>
  </div>
</div>

<style>
  .page {
    --measure: 720px;
    height: 100%;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .page.wide {
    --measure: 980px;
  }
  /*
   * The scroll area always reserves the scrollbar gutter; the toolbar reserves the same, so titles
   * and toolbar buttons line up with the content below.
   */
  .toolbar {
    flex: none;
    height: 52px;
    padding-right: 0;
  }
  .bar {
    height: 100%;
    max-width: var(--measure);
    margin: 0 auto;
    padding: 0 var(--gutter);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  h1 {
    font: 600 15px/1 var(--font);
    color: var(--label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .scroll {
    flex: 1;
    overflow-y: scroll;
    scrollbar-gutter: stable;
    overflow-x: hidden;
  }
  /* Sections rise in one after another when a page opens. */
  .content > :global(*) {
    animation: enter 460ms var(--ease-out) backwards;
  }
  .content > :global(:nth-child(2)) {
    animation-delay: 50ms;
  }
  .content > :global(:nth-child(3)) {
    animation-delay: 100ms;
  }
  .content > :global(:nth-child(4)) {
    animation-delay: 150ms;
  }
  .content > :global(:nth-child(n + 5)) {
    animation-delay: 200ms;
  }
  .content {
    max-width: var(--measure);
    margin: 0 auto;
    padding: 4px var(--gutter) 40px;
    display: flex;
    flex-direction: column;
    gap: 26px;
  }
  :global(:root:not([data-vibrancy])) .toolbar {
    padding-right: 11px;
  }
</style>
