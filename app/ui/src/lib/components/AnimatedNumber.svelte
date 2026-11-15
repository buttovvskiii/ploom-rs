<script lang="ts">
  /** A number that glides to its new value instead of jumping. */
  import { Tween } from "svelte/motion";
  import { easeOut, reducedMotion } from "$lib/motion";

  interface Props {
    value: number | null;
    /** Decimal places to show. */
    digits?: number;
    duration?: number;
    /** Shown while the value is unknown. */
    placeholder?: string;
    format?: (value: number) => string;
  }
  let { value, digits = 0, duration = 520, placeholder = "—", format }: Props = $props();

  const tween = new Tween(0, { duration: 0, easing: easeOut });
  let known = false;

  $effect(() => {
    if (value === null) {
      known = false;
      return;
    }
    // The first reading appears as is; later ones travel.
    const animate = known && !reducedMotion();
    known = true;
    void tween.set(value, { duration: animate ? duration : 0 });
  });

  const shown = $derived.by(() => {
    if (value === null) return placeholder;
    const v = Number(tween.current.toFixed(digits));
    return format ? format(v) : v.toFixed(digits);
  });
</script>

<span class="num">{shown}</span>
