<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Spinner from "./Spinner.svelte";

  interface Props extends HTMLButtonAttributes {
    variant?: "primary" | "secondary" | "plain" | "destructive";
    size?: "small" | "regular" | "large";
    loading?: boolean;
    children: Snippet;
  }

  let {
    variant = "secondary",
    size = "regular",
    loading = false,
    disabled = false,
    type = "button",
    children,
    ...rest
  }: Props = $props();
</script>

<button
  {...rest}
  {type}
  class="button {variant} {size}"
  disabled={disabled || loading}
  aria-busy={loading || undefined}
>
  {#if loading}
    <Spinner size={size === "small" ? 11 : 13} />
  {/if}
  {@render children()}
</button>

<style>
  .button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    flex: none;
    border: 0;
    border-radius: var(--radius-control);
    font: 500 13px/1 var(--font);
    white-space: nowrap;
    transition:
      background-color var(--dur-fast) ease,
      color var(--dur-fast) ease,
      opacity var(--dur-fast) ease,
      transform var(--dur-fast) var(--ease-out);
  }

  .regular {
    height: 28px;
    padding: 0 14px;
  }
  .small {
    height: 24px;
    padding: 0 10px;
    font-size: 12px;
  }
  .large {
    height: 32px;
    padding: 0 18px;
    border-radius: 8px;
  }

  .secondary {
    background: var(--control);
    color: var(--label);
    box-shadow:
      inset 0 0.5px 0 rgba(255, 255, 255, 0.1),
      0 0.5px 1px rgba(0, 0, 0, 0.3);
  }
  .secondary:hover:not(:disabled) {
    background: var(--control-hover);
  }

  .primary {
    background: var(--accent-strong);
    color: var(--on-accent);
    font-weight: 600;
    box-shadow: 0 0.5px 1px rgba(0, 0, 0, 0.35);
  }
  .primary:hover:not(:disabled) {
    background: #fff;
  }

  .destructive {
    background: var(--control);
    color: var(--red);
    font-weight: 600;
    box-shadow: inset 0 0.5px 0 rgba(255, 255, 255, 0.1);
  }
  .destructive:hover:not(:disabled) {
    background: var(--control-hover);
  }

  .plain {
    background: transparent;
    color: var(--label-2);
    padding-inline: 8px;
  }
  .plain:hover:not(:disabled) {
    color: var(--label);
    background: var(--fill);
  }

  .button:active:not(:disabled) {
    transform: scale(0.97);
  }

  .button:disabled {
    opacity: 0.4;
  }
  .button[aria-busy="true"] {
    opacity: 0.75;
  }
</style>
