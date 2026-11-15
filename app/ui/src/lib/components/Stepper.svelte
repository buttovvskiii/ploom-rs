<script lang="ts">
  import { onDestroy } from "svelte";

  interface Props {
    value: number;
    min: number;
    max: number;
    step?: number;
    label: string;
    decreaseLabel: string;
    increaseLabel: string;
    /** Width of the number field in `ch`. */
    width?: number;
    disabled?: boolean;
    onchange: (value: number) => void;
  }

  let {
    value,
    min,
    max,
    step = 1,
    label,
    decreaseLabel,
    increaseLabel,
    width = 2,
    disabled = false,
    onchange,
  }: Props = $props();

  let text = $state("");
  let editing = $state(false);
  let hold: ReturnType<typeof setTimeout> | undefined;

  const shown = $derived(editing ? text : String(value));
  const canDown = $derived(!disabled && value > min);
  const canUp = $derived(!disabled && value < max);

  function clamp(n: number): number {
    return Math.min(max, Math.max(min, n));
  }

  function set(next: number): void {
    const n = clamp(next);
    if (n !== value) onchange(n);
  }

  function nudge(delta: number): void {
    // Apply a typed value first, or the later blur would overwrite this step.
    commit();
    set(value + delta * step);
  }

  function press(delta: number, event: PointerEvent): void {
    if (event.button !== 0) return;
    nudge(delta);
    let current = value + delta * step;
    const repeat = (): void => {
      current = clamp(current + delta * step);
      set(current);
      if (current > min && current < max) hold = setTimeout(repeat, 70);
    };
    hold = setTimeout(repeat, 420);
  }

  function release(): void {
    clearTimeout(hold);
  }

  function commit(): void {
    if (!editing) return;
    editing = false;
    const n = Number.parseInt(text, 10);
    if (Number.isFinite(n)) set(Math.round(n / step) * step);
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      nudge(event.key === "ArrowUp" ? 1 : -1);
    } else if (event.key === "Enter") {
      commit();
    } else if (event.key === "Escape") {
      editing = false;
    }
  }

  // Keyboard activation; pointer presses are handled by `press`.
  function onclick(delta: number, event: MouseEvent): void {
    if (event.detail === 0) nudge(delta);
  }

  onDestroy(release);
</script>

<span class="stepper" class:disabled>
  <button
    type="button"
    aria-label={decreaseLabel}
    disabled={!canDown}
    onpointerdown={(e) => press(-1, e)}
    onpointerup={release}
    onpointerleave={release}
    onpointercancel={release}
    onclick={(e) => onclick(-1, e)}
  >
    <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 6h7" /></svg>
  </button>
  <input
    class="num"
    type="text"
    inputmode="numeric"
    role="spinbutton"
    aria-label={label}
    aria-valuemin={min}
    aria-valuemax={max}
    aria-valuenow={value}
    autocomplete="off"
    style:width="calc({width}ch + 14px)"
    value={shown}
    {disabled}
    onfocus={(e) => {
      text = String(value);
      editing = true;
      e.currentTarget.select();
    }}
    oninput={(e) => {
      text = e.currentTarget.value.replace(/\D/g, "").slice(0, 4);
      editing = true;
    }}
    onblur={commit}
    {onkeydown}
  />
  <button
    type="button"
    aria-label={increaseLabel}
    disabled={!canUp}
    onpointerdown={(e) => press(1, e)}
    onpointerup={release}
    onpointerleave={release}
    onpointercancel={release}
    onclick={(e) => onclick(1, e)}
  >
    <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 6h7M6 2.5v7" /></svg>
  </button>
</span>

<style>
  .stepper {
    display: inline-flex;
    align-items: stretch;
    height: 30px;
    border-radius: 8px;
    background: var(--control);
    box-shadow:
      inset 0 0.5px 0 rgba(255, 255, 255, 0.1),
      0 0.5px 1px rgba(0, 0, 0, 0.3);
    vertical-align: middle;
  }
  .stepper.disabled {
    opacity: 0.45;
  }
  button {
    display: grid;
    place-items: center;
    width: 30px;
    border: 0;
    background: transparent;
    color: var(--label);
    transition:
      background-color var(--dur-fast) ease,
      opacity var(--dur-fast) ease;
  }
  button:first-child {
    border-radius: 8px 0 0 8px;
  }
  button:last-child {
    border-radius: 0 8px 8px 0;
  }
  button:hover:not(:disabled) {
    background: var(--control-hover);
  }
  button:active:not(:disabled) {
    background: var(--fill-strong);
  }
  button:disabled {
    opacity: 0.3;
  }
  svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
  }
  input {
    min-width: 0;
    padding: 0;
    border: 0;
    border-inline: 0.5px solid var(--hairline-strong);
    background: transparent;
    color: var(--label);
    font: 600 15px/1 var(--font);
    text-align: center;
  }
  input:focus-visible {
    outline: none;
    background: var(--fill);
  }
  .stepper:has(input:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
</style>
