<script lang="ts" generics="T extends string">
  interface Option {
    value: T;
    label: string;
  }

  interface Props {
    options: Option[];
    value: T;
    label: string;
    size?: "small" | "regular";
    disabled?: boolean;
    onchange?: (value: T) => void;
  }

  let {
    options,
    value = $bindable(),
    label,
    size = "regular",
    disabled = false,
    onchange,
  }: Props = $props();

  const index = $derived(
    Math.max(
      0,
      options.findIndex((o) => o.value === value),
    ),
  );
  let buttons: HTMLButtonElement[] = $state([]);

  function select(next: T): void {
    if (disabled || next === value) return;
    value = next;
    onchange?.(next);
  }

  function onkeydown(event: KeyboardEvent): void {
    const delta = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (!delta) return;
    event.preventDefault();
    const nextIndex = (index + delta + options.length) % options.length;
    const next = options[nextIndex];
    if (next) {
      select(next.value);
      buttons[nextIndex]?.focus();
    }
  }
</script>

<div
  class="segmented {size}"
  class:disabled
  role="radiogroup"
  aria-label={label}
  style:--count={options.length}
  style:--index={index}
>
  <span class="thumb" aria-hidden="true"></span>
  {#each options as option, i (option.value)}
    <button
      bind:this={buttons[i]}
      type="button"
      role="radio"
      aria-checked={option.value === value}
      tabindex={option.value === value ? 0 : -1}
      {disabled}
      onclick={() => select(option.value)}
      {onkeydown}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--count), minmax(0, 1fr));
    padding: 2px;
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.07);
    box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.05);
  }
  .thumb {
    position: absolute;
    top: 2px;
    bottom: 2px;
    left: 2px;
    width: calc((100% - 4px) / var(--count));
    border-radius: 5px;
    background: #5d5d61;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.3),
      inset 0 0.5px 0 rgba(255, 255, 255, 0.12);
    transform: translateX(calc(var(--index) * 100%));
    transition: transform 300ms var(--ease-out);
  }
  button {
    position: relative;
    z-index: 1;
    min-width: 0;
    padding: 0 10px;
    border: 0;
    background: transparent;
    color: var(--label-2);
    font: 500 12px/1 var(--font);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color var(--dur-fast) ease;
  }
  .regular button {
    height: 24px;
  }
  .small button {
    height: 20px;
    font-size: 11px;
    padding: 0 8px;
  }
  button[aria-checked="true"] {
    color: var(--label);
  }
  button:hover:not([aria-checked="true"]):not(:disabled) {
    color: var(--label);
  }
  button:focus-visible {
    outline-offset: -1px;
    border-radius: 5px;
  }
  .disabled {
    opacity: 0.4;
  }
</style>
