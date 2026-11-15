<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import { formatCountdown, formatTime } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/index.svelte";
  import { clock } from "$lib/state/clock.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";
  import { limitPhase, sticksLeft } from "./limit";
  import StickMarks from "./StickMarks.svelte";

  interface Props {
    onenable: () => void;
    enabling?: boolean;
  }
  let { onenable, enabling = false }: Props = $props();

  const t = $derived(i18n.t.limit);
  const limit = $derived(snapshot.current.limit);
  const phase = $derived(limitPhase(limit, clock.now));
  const total = $derived(limit.settings.sticks);
  const used = $derived(Math.min(limit.sticksUsed, total));
  const span = $derived(t.span(limit.settings.cooldownMinutes));

  const remainingS = $derived(
    limit.lockedUntilMs === null ? 0 : (limit.lockedUntilMs - clock.now) / 1000,
  );
  const elapsed = $derived(
    phase === "cooldown" ? 1 - Math.min(1, remainingS / (limit.settings.cooldownMinutes * 60)) : 1,
  );
  const resetsAt = $derived(
    limit.windowStartedAtMs === null
      ? null
      : limit.windowStartedAtMs + limit.settings.windowHours * 3600_000,
  );
</script>

<section class="status" aria-live="polite">
  {#if phase === "off"}
    <p class="lead">{t.offLead}</p>
    <div>
      <Button variant="primary" size="large" loading={enabling} onclick={onenable}
        >{t.turnOn}</Button
      >
    </div>
  {:else if phase === "cooldown"}
    <div class="head">
      <div>
        <p class="caption">{t.breakTitle}</p>
        <p class="figure num" aria-label={t.remainingLabel}>{formatCountdown(remainingS)}</p>
      </div>
      <p class="until num">{t.breakUntil(formatTime(limit.lockedUntilMs ?? 0))}</p>
    </div>
    <div class="track" aria-hidden="true"><span style:transform="scaleX({elapsed})"></span></div>
    <p class="line">{t.breakNote}</p>
  {:else if phase === "pending"}
    <p class="caption">{t.pendingTitle}</p>
    <p class="lead">{t.pendingBody}</p>
  {:else}
    <div class="head">
      <div>
        <p class="caption">{t.usedCaption}</p>
        <p class="figure num">{t.used(used, total)}</p>
      </div>
    </div>
    <StickMarks {used} {total} label={t.usedLabel(used, total)} />
    <p class="line" class:alert={phase === "reached"}>
      {phase === "reached" ? t.reached : t.leftThen(sticksLeft(limit), span)}
      {#if phase === "counting"}
        <span class="quiet">{resetsAt === null ? t.fresh : t.resets(formatTime(resetsAt))}</span>
      {/if}
    </p>
  {/if}
</section>

<style>
  .status {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 22px 24px 24px;
    border-radius: var(--radius-group);
    background: var(--group);
    box-shadow: var(--surface-edge);
  }
  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
  }
  .caption {
    font: var(--text-callout);
    color: var(--label-2);
  }
  .figure {
    margin-top: 2px;
    font: 300 48px/1.05 var(--font);
    letter-spacing: -0.025em;
  }
  .until {
    padding-bottom: 7px;
    font: var(--text-headline);
    color: var(--label-2);
  }
  .lead {
    max-width: 56ch;
    font: 400 15px/1.45 var(--font);
    color: var(--label);
  }
  .line {
    max-width: 62ch;
    font: 400 14px/1.45 var(--font);
    color: var(--label);
  }
  .line.alert {
    color: var(--orange);
  }
  .quiet {
    display: block;
    margin-top: 2px;
    font: var(--text-callout);
    color: var(--label-3);
  }
  .track {
    height: 3px;
    border-radius: 2px;
    background: var(--fill-strong);
    overflow: hidden;
  }
  .track span {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--label-2);
    transform-origin: left;
    transition: transform 1s linear;
  }
  @media (prefers-reduced-motion: reduce) {
    .track span {
      transition: none;
    }
  }
</style>
