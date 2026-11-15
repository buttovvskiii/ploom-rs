<script lang="ts">
  import { toasts } from "$lib/state/toasts.svelte";
  import Icon from "./icons/Icon.svelte";
  import { cubicOut } from "svelte/easing";

  function rise(_node: Element) {
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) return { duration: 0 };
    return {
      duration: 220,
      easing: cubicOut,
      css: (t: number) => `opacity:${t};transform:translateY(${(1 - t) * 8}px)`,
    };
  }
</script>

<div class="toaster" role="status" aria-live="polite">
  {#each toasts.items as toast (toast.id)}
    <button
      type="button"
      class="toast"
      class:error={toast.tone === "error"}
      title={toast.detail ?? undefined}
      onclick={() => toasts.dismiss(toast.id)}
      transition:rise
    >
      {#if toast.tone === "error"}<Icon name="warning" size={15} />{/if}
      <span>{toast.message}</span>
    </button>
  {/each}
</div>

<style>
  .toaster {
    position: fixed;
    left: calc(var(--sidebar-width) + (100vw - var(--sidebar-width)) / 2);
    bottom: 20px;
    z-index: 60;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: min(460px, calc(100vw - var(--sidebar-width) - 48px));
    padding: 9px 14px;
    border: 0;
    border-radius: 10px;
    background: rgba(58, 58, 62, 0.94);
    -webkit-backdrop-filter: blur(20px);
    backdrop-filter: blur(20px);
    color: var(--label);
    font: var(--text-body);
    text-align: left;
    box-shadow:
      0 0 0 0.5px rgba(0, 0, 0, 0.5),
      inset 0 0 0 0.5px rgba(255, 255, 255, 0.1),
      0 10px 30px rgba(0, 0, 0, 0.4);
  }
  .error :global(.icon) {
    color: var(--orange);
  }
</style>
