<script lang="ts" generics="T extends string | number">
  interface Option {
    value: T;
    label: string;
    disabled?: boolean;
  }

  interface Props {
    options: Option[];
    value: T;
    label: string;
    disabled?: boolean;
    onchange: (value: T) => void;
  }

  let { options, value, label, disabled = false, onchange }: Props = $props();

  let buttons: HTMLButtonElement[] = $state([]);
  const selected = $derived(options.findIndex((o) => o.value === value));
  const firstEnabled = $derived(options.findIndex((o) => !o.disabled));

  function off(option: Option): boolean {
    return disabled || option.disabled === true;
  }

  function onkeydown(event: KeyboardEvent, from: number): void {
    const delta =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? 1
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? -1
          : 0;
    if (!delta) return;
    event.preventDefault();
    for (let n = 1; n <= options.length; n++) {
      const i = (from + delta * n + options.length * n) % options.length;
      const option = options[i];
      if (option && !off(option)) {
        buttons[i]?.focus();
        if (option.value !== value) onchange(option.value);
        return;
      }
    }
  }
</script>

<div class="chips" role="radiogroup" aria-label={label}>
  {#each options as option, i (option.value)}
    <button
      bind:this={buttons[i]}
      type="button"
      role="radio"
      class="num"
      aria-checked={option.value === value}
      tabindex={i === (selected >= 0 ? selected : firstEnabled) ? 0 : -1}
      disabled={off(option)}
      onclick={() => option.value !== value && onchange(option.value)}
      onkeydown={(e) => onkeydown(e, i)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  button {
    height: 30px;
    padding: 0 13px;
    border: 0;
    border-radius: 15px;
    background: var(--fill);
    color: var(--label-2);
    font: 500 13px/1 var(--font);
    white-space: nowrap;
    box-shadow: inset 0 0 0 0.5px var(--hairline);
    transition:
      background-color var(--dur-fast) ease,
      color var(--dur-fast) ease,
      transform var(--dur-fast) var(--ease-out);
  }
  button:hover:not(:disabled):not([aria-checked="true"]) {
    background: var(--fill-strong);
    color: var(--label);
  }
  button:active:not(:disabled) {
    transform: scale(0.96);
  }
  button[aria-checked="true"] {
    background: var(--accent-strong);
    color: var(--on-accent);
    font-weight: 600;
    box-shadow: none;
  }
  button:disabled {
    opacity: 0.35;
  }
  button:focus-visible {
    border-radius: 15px;
  }
</style>
