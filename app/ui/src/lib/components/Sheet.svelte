<script lang="ts">
  import { tick, type Snippet } from "svelte";
  import { cubicOut } from "svelte/easing";

  interface Props {
    open: boolean;
    /** Accessible title (also rendered unless `hideTitle`). */
    title: string;
    /** Called on Escape or backdrop click when `dismissable`. */
    onclose: () => void;
    dismissable?: boolean;
    width?: number;
    children: Snippet;
    footer?: Snippet;
  }

  let { open, title, onclose, dismissable = true, width = 440, children, footer }: Props = $props();

  const id = $props.id();
  let panel: HTMLDivElement | undefined = $state();
  let restoreFocus: HTMLElement | null = null;

  const reduced = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

  /** macOS sheets drop from the title bar: a short slide with a settle. */
  function present(_node: Element, { out = false } = {}) {
    if (reduced()) return { duration: 0 };
    return {
      duration: out ? 160 : 280,
      easing: cubicOut,
      css: (t: number) =>
        `opacity:${Math.min(1, t * 1.6)};transform:translateY(${(1 - t) * -18}px) scale(${0.985 + t * 0.015})`,
    };
  }

  function fade(_node: Element, { duration = 200 } = {}) {
    if (reduced()) return { duration: 0 };
    return { duration, css: (t: number) => `opacity:${t}` };
  }

  const FOCUSABLE =
    'button:not([disabled]), [href], input:not([disabled]), select, textarea, [tabindex]:not([tabindex="-1"])';

  $effect(() => {
    if (!open) return;
    restoreFocus = document.activeElement as HTMLElement | null;
    void tick().then(() => {
      const preferred = panel?.querySelector<HTMLElement>("[data-autofocus]");
      (preferred ?? panel?.querySelector<HTMLElement>(FOCUSABLE) ?? panel)?.focus();
    });
    return () => restoreFocus?.focus?.();
  });

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "Escape" && dismissable) {
      event.stopPropagation();
      onclose();
      return;
    }
    if (event.key !== "Tab" || !panel) return;
    const items = [...panel.querySelectorAll<HTMLElement>(FOCUSABLE)];
    const first = items[0];
    const last = items.at(-1);
    if (!first || !last) {
      event.preventDefault();
      return;
    }
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }
</script>

{#if open}
  <div
    class="backdrop"
    transition:fade
    role="presentation"
    onclick={() => dismissable && onclose()}
  ></div>
  <div class="anchor">
    <div
      bind:this={panel}
      class="sheet"
      style:width="{width}px"
      role="dialog"
      aria-modal="true"
      aria-labelledby="{id}-title"
      tabindex="-1"
      {onkeydown}
      in:present
      out:present={{ out: true }}
    >
      <h2 id="{id}-title" class="title">{title}</h2>
      <div class="body">
        {@render children()}
      </div>
      {#if footer}
        <div class="footer">{@render footer()}</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: rgba(0, 0, 0, 0.32);
  }
  .anchor {
    position: fixed;
    inset: var(--titlebar) 0 0 0;
    z-index: 51;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 12px 16px;
    pointer-events: none;
  }
  .sheet {
    pointer-events: auto;
    max-width: 100%;
    max-height: calc(100vh - var(--titlebar) - 40px);
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    background: var(--sheet);
    box-shadow:
      0 0 0 0.5px rgba(0, 0, 0, 0.6),
      inset 0 0 0 0.5px rgba(255, 255, 255, 0.1),
      0 22px 50px rgba(0, 0, 0, 0.5),
      0 4px 12px rgba(0, 0, 0, 0.3);
    outline: none;
  }
  .title {
    padding: 20px 22px 0;
    font: var(--text-title);
  }
  .body {
    padding: 10px 22px 4px;
    overflow: auto;
    color: var(--label-2);
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 22px 18px;
  }
</style>
