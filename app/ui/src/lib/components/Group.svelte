<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title?: string;
    /** Footnote under the group; string or rich snippet. */
    footer?: string | Snippet;
    /** Trailing header content (a button, a status). */
    actions?: Snippet;
    children: Snippet;
  }

  let { title, footer, actions, children }: Props = $props();
  const id = $props.id();
</script>

<section class="section" aria-labelledby={title ? `${id}-title` : undefined}>
  {#if title || actions}
    <header>
      {#if title}<h2 id="{id}-title">{title}</h2>{/if}
      {#if actions}<div class="actions">{@render actions()}</div>{/if}
    </header>
  {/if}
  <div class="group">
    {@render children()}
  </div>
  {#if footer}
    <div class="footer">
      {#if typeof footer === "string"}<p>{footer}</p>{:else}{@render footer()}{/if}
    </div>
  {/if}
</section>

<style>
  .section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 24px;
    padding: 0 2px 0 4px;
  }
  h2 {
    font: var(--text-headline);
    color: var(--label);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .group {
    border-radius: var(--radius-group);
    background: var(--group);
    box-shadow: var(--surface-edge);
    overflow: hidden;
  }
  .footer {
    padding: 0 4px;
    font: var(--text-callout);
    color: var(--label-2);
    max-width: 62ch;
  }
</style>
