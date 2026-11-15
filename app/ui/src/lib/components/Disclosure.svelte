<script lang="ts">
  import type { Snippet } from "svelte";
  import { slide } from "svelte/transition";
  import { easeOut, reducedMotion } from "$lib/motion";

  interface Props {
    title: string;
    open?: boolean;
    children: Snippet;
  }
  let { title, open = $bindable(false), children }: Props = $props();
  const id = $props.id();
</script>

<div class="disclosure" class:open>
  <button type="button" aria-expanded={open} aria-controls={id} onclick={() => (open = !open)}>
    <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M3.5 1.5 7 5 3.5 8.5" /></svg>
    {title}
  </button>
  {#if open}
    <div
      {id}
      class="body"
      transition:slide={{ duration: reducedMotion() ? 0 : 240, easing: easeOut }}
    >
      {@render children()}
    </div>
  {/if}
</div>

<style>
  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 6px 4px 2px;
    border: 0;
    background: transparent;
    color: var(--label-2);
    font: var(--text-callout);
    transition: color var(--dur-fast) ease;
  }
  button:hover {
    color: var(--label);
  }
  svg {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform var(--dur) var(--ease-out);
  }
  .open svg {
    transform: rotate(90deg);
  }
  .body {
    padding-top: 10px;
  }
  @media (prefers-reduced-motion: reduce) {
    svg {
      transition: none;
    }
  }
</style>
