<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    disabled?: boolean;
    pending?: boolean;
    onchange?: (checked: boolean) => void;
  }

  let { checked, label, disabled = false, pending = false, onchange }: Props = $props();
</script>

<button
  type="button"
  role="switch"
  class="switch"
  class:on={checked}
  class:pending
  aria-checked={checked}
  aria-label={label}
  aria-busy={pending || undefined}
  {disabled}
  onclick={() => onchange?.(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .switch {
    position: relative;
    flex: none;
    width: 34px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.09);
    box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.07);
    transition:
      background-color var(--dur) ease,
      box-shadow var(--dur) ease;
  }
  .switch:focus-visible {
    border-radius: 10px;
  }
  .on {
    background: color-mix(in srgb, var(--accent) 24%, transparent);
    box-shadow: inset 0 0 0 0.5px color-mix(in srgb, var(--accent) 40%, transparent);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #8e8e93;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.35),
      0 0 0 0.5px rgba(0, 0, 0, 0.15);
    transition:
      transform 300ms var(--ease-spring),
      width var(--dur-fast) var(--ease-out),
      background-color var(--dur) ease;
  }
  .on .knob {
    transform: translateX(14px);
    background: var(--accent);
  }
  .switch:active:not(:disabled) .knob {
    width: 19px;
  }
  .on:active:not(:disabled) .knob {
    transform: translateX(11px);
  }
  .pending {
    opacity: 0.6;
  }
  .switch:disabled {
    opacity: 0.35;
  }
</style>
