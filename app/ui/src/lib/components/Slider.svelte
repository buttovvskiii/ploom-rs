<script lang="ts">
  interface Props {
    value: number;
    min: number;
    max: number;
    step?: number;
    label: string;
    valueText?: string;
    tint?: string;
    disabled?: boolean;
  }

  let {
    value = $bindable(),
    min,
    max,
    step = 1,
    label,
    valueText,
    tint = "var(--accent)",
    disabled = false,
  }: Props = $props();

  const fraction = $derived(max > min ? (value - min) / (max - min) : 0);
</script>

<input
  class="slider"
  type="range"
  {min}
  {max}
  {step}
  {disabled}
  bind:value
  aria-label={label}
  aria-valuetext={valueText}
  style:--fraction={fraction}
  style:--tint={tint}
/>

<style>
  .slider {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 20px;
    margin: 0;
    background: transparent;
  }
  .slider::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 2px;
    background: linear-gradient(
      to right,
      var(--tint) calc(var(--fraction) * 100%),
      rgba(255, 255, 255, 0.12) calc(var(--fraction) * 100%)
    );
  }
  .slider::-moz-range-track {
    height: 4px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.12);
  }
  .slider::-moz-range-progress {
    height: 4px;
    border-radius: 2px;
    background: var(--tint);
  }
  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 18px;
    height: 18px;
    margin-top: -7px;
    border-radius: 50%;
    background: #ececee;
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.45),
      0 0 0 0.5px rgba(0, 0, 0, 0.2);
    transition: transform 120ms ease;
  }
  .slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border: 0;
    border-radius: 50%;
    background: #ececee;
  }
  .slider:active::-webkit-slider-thumb {
    transform: scale(1.06);
  }
  .slider:focus-visible {
    outline: none;
  }
  .slider:focus-visible::-webkit-slider-thumb {
    box-shadow:
      0 0 0 3px var(--focus),
      0 1px 3px rgba(0, 0, 0, 0.45);
  }
  .slider:disabled {
    opacity: 0.4;
  }
</style>
