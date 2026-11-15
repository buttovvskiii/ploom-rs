<script lang="ts">
  /** The 3D device. */
  import { onMount } from "svelte";
  import type { Snapshot } from "$lib/api";
  import { i18n } from "$lib/i18n/index.svelte";
  import { visualTargets, type Palette } from "./behavior";
  import DeviceGlyph from "./DeviceGlyph.svelte";
  import type { FinishId } from "./finishes";
  import {
    acquireViewer,
    isViewerRevealed,
    markViewerRevealed,
    webglAvailable,
    type ViewerHandle,
  } from "./host";

  interface Props {
    snapshot: Snapshot;
    finish: FinishId;
    /** Set when a custom GLB replaced the built-in model (finishes don't apply). */
    custom?: boolean;
  }
  let { snapshot, finish, custom = $bindable(false) }: Props = $props();

  const t = $derived(i18n.t.device3d);
  const motionQuery = matchMedia("(prefers-reduced-motion: reduce)");
  let reducedMotion = $state(motionQuery.matches);
  // Shown before: appear at once. First time: fade and settle in.
  const entrance = !isViewerRevealed();
  let phase = $state<"loading" | "ready" | "fallback">(entrance ? "loading" : "ready");
  let engine = $state.raw<ViewerHandle | null>(null);
  let host: HTMLDivElement | undefined = $state();

  const palette = readPalette();
  const targets = $derived(visualTargets(snapshot, palette, reducedMotion));
  const label = $derived(
    custom || finish === "neutral" ? t.labelCustom : t.label(t.finishes[finish]),
  );

  $effect(() => engine?.setTargets(targets));
  $effect(() => engine?.setFinish(finish));
  $effect(() => engine?.setReducedMotion(reducedMotion));

  onMount(() => {
    const el = host;
    if (!el) return;

    if (!webglAvailable()) {
      phase = "fallback";
      return;
    }

    const handle = acquireViewer({
      host: el,
      finish,
      reducedMotion,
      onReady: () => {
        if (phase !== "loading") return;
        // Two frames, so the hidden canvas is on screen before the transition starts.
        requestAnimationFrame(() =>
          requestAnimationFrame(() => {
            if (phase !== "loading") return;
            phase = "ready";
            markViewerRevealed();
          }),
        );
      },
      onLost: () => {
        phase = "fallback";
        engine = null;
      },
      onCustom: (value) => (custom = value),
    });
    engine = handle;

    let onScreen = false;
    const sync = () => handle.setActive(onScreen && document.visibilityState === "visible");
    const onMotion = () => (reducedMotion = motionQuery.matches);
    motionQuery.addEventListener("change", onMotion);
    document.addEventListener("visibilitychange", sync);
    const io = new IntersectionObserver((entries) => {
      onScreen = entries.some((e) => e.isIntersecting);
      sync();
    });
    io.observe(el);
    const ro = new ResizeObserver(() => handle.resize(el.clientWidth, el.clientHeight));
    ro.observe(el);

    return () => {
      motionQuery.removeEventListener("change", onMotion);
      document.removeEventListener("visibilitychange", sync);
      io.disconnect();
      ro.disconnect();
      handle.release();
      engine = null;
    };
  });

  function readPalette(): Palette {
    const css = getComputedStyle(document.documentElement);
    const token = (name: string, fallback: string) => css.getPropertyValue(name).trim() || fallback;
    return {
      modes: {
        delicate: token("--tint-delicate", "#c2b36f"),
        balanced: token("--tint-balanced", "#d39a55"),
        intense: token("--tint-intense", "#cf5c62"),
        batterySaver: token("--tint-batterySaver", "#6f98c8"),
      },
      charging: token("--green", "#32d74b"),
      heat: "#ff7a2e",
      find: "#e9edf3",
    };
  }
</script>

<div
  class="viewer {phase}"
  class:entrance
  class:asleep={targets.dim > 0}
  bind:this={host}
  role="img"
  aria-label={label}
>
  {#if phase === "fallback"}
    <div class="still" aria-hidden="true">
      <DeviceGlyph
        {finish}
        detailed
        tone={targets.status.intensity > 0 ? targets.status.color : null}
      />
    </div>
  {:else if entrance}
    <div class="loading" aria-hidden="true"></div>
  {/if}
</div>

<style>
  .viewer {
    position: relative;
    width: 100%;
    height: 100%;
    contain: strict;
  }
  /* A faint pool of light on the floor; the scene's contact shadow sits in it. */
  .viewer::before {
    content: "";
    position: absolute;
    left: 6%;
    right: 6%;
    bottom: 1%;
    height: 22%;
    background: radial-gradient(
      closest-side,
      rgba(255, 255, 255, 0.055),
      rgba(255, 255, 255, 0.018) 55%,
      transparent
    );
    transition: opacity 700ms var(--ease-out);
  }
  .asleep::before {
    opacity: 0.5;
  }
  /* The canvas is appended at runtime, so it is styled globally within .viewer. */
  .viewer :global(canvas) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    opacity: 0;
    transform: translateY(10px) scale(0.975);
    cursor: grab;
    touch-action: pan-y;
  }
  .ready :global(canvas) {
    opacity: 1;
    transform: none;
  }
  .entrance :global(canvas) {
    transition:
      opacity 900ms var(--ease-in-out),
      transform 1100ms var(--ease-out);
  }
  /* The canvas in transit between two viewers (see host.ts). */
  :global(.device-flight) {
    position: fixed;
    z-index: 4;
    pointer-events: none;
  }
  :global(.device-flight canvas) {
    display: block;
    width: 100%;
    height: 100%;
  }
  .fallback :global(canvas) {
    display: none;
  }

  /* A quiet shimmer while three.js loads — abstract, never a drawing of the device. */
  .loading {
    position: absolute;
    inset: 0;
    background: radial-gradient(42% 36% at 50% 46%, rgba(255, 255, 255, 0.05), transparent 70%);
    opacity: 0;
    /* Only a slow start gets the shimmer; it leaves as the device arrives. */
    animation: breathe 2.4s var(--ease-in-out) 350ms infinite;
    transition: opacity 500ms var(--ease-out);
  }
  .ready .loading {
    animation: none;
  }
  @keyframes breathe {
    0%,
    100% {
      opacity: 0.4;
    }
    50% {
      opacity: 0.9;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .loading {
      animation: none;
    }
    .viewer :global(canvas) {
      transform: none;
    }
    .entrance :global(canvas) {
      transition: opacity 300ms linear;
    }
  }

  /* Static stand-in, only when WebGL is unavailable. */
  .still {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 9% 0 12%;
    transition: filter 700ms var(--ease-out);
  }
  .asleep .still {
    filter: saturate(0.5) brightness(0.62);
  }
</style>
