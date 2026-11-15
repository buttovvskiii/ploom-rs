<script lang="ts">
  import CurvePlot from "$lib/components/CurvePlot.svelte";
  import { customCurve, domainFor, stepRange, totalDuration } from "$lib/curve/curve";
  import { formatClock, formatDuration } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/index.svelte";
  import { modeName, tintOf } from "$lib/modes";
  import { catalog } from "$lib/state/catalog.svelte";
  import { clock } from "$lib/state/clock.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";

  const t = $derived(i18n.t);
  const status = $derived(snapshot.status);
  const mode = $derived(catalog.mode(status.heatingMode));
  const custom = $derived(status.customProfile);

  const curve = $derived.by(() => {
    if (!mode) return null;
    if (custom) return customCurve(mode, custom.holdTempC, custom.holdSecs);
    return mode.curve;
  });
  const domain = $derived(
    domainFor((catalog.current?.modes ?? []).flatMap((m) => (m.curve ? [m.curve] : []))),
  );
  const highlight = $derived(
    custom && curve && mode?.holdStepIndex != null ? stepRange(curve, mode.holdStepIndex) : null,
  );

  const started = $derived(status.sessionStartedAtMs);
  const elapsed = $derived(started === null ? null : Math.max(0, (clock.now - started) / 1000));

  // Smooth playhead only while a session runs.
  $effect(() => {
    if (started === null) return;
    return clock.precise();
  });

  const state = $derived.by((): { text: string; tone: "live" | "warm" | "quiet" } => {
    const c = t.overview.curve;
    if (status.activity === "preheating") return { text: c.preheating, tone: "live" };
    if (status.activity === "heating") return { text: c.heating, tone: "live" };
    if (status.temperatureWarning) return { text: c.warm, tone: "warm" };
    if (status.activity === "charging") return { text: t.activity.charging, tone: "quiet" };
    return { text: c.ready, tone: "quiet" };
  });

  // Balanced has no readable curve; its nominal length (approximate) is the only time reference
  // there is.
  const nominalS = $derived(mode && !curve ? mode.minutes * 60 : null);
  const progress = $derived(
    elapsed !== null && nominalS ? Math.min(1, elapsed / nominalS) : started === null ? 0 : null,
  );

  const timing = $derived.by(() => {
    if (elapsed === null) return null;
    if (curve)
      return t.overview.curve.elapsed(formatClock(elapsed), formatClock(totalDuration(curve)));
    if (nominalS)
      return t.overview.curve.approxElapsed(formatClock(elapsed), formatClock(nominalS));
    return formatClock(elapsed);
  });
</script>

<section
  class="panel"
  aria-label={t.overview.curve.label}
  style:--tint={tintOf(status.heatingMode)}
>
  <header>
    <div class="title">
      <h2>{status.heatingMode ? modeName(status.heatingMode) : t.overview.curve.label}</h2>
      {#if custom}
        <p class="sub">
          {t.overview.curve.custom(custom.holdTempC, formatDuration(custom.holdSecs))}
        </p>
      {:else if mode}
        <p class="sub">
          {t.heating.sticks(mode.maxSticks)} · {t.heating.minutes(mode.minutes)}
        </p>
      {/if}
    </div>
    <div
      class="state {state.tone}"
      title={state.tone === "warm" ? t.overview.curve.warmHint : undefined}
    >
      <span class="state-text">
        {#if state.tone !== "quiet"}<span class="pip" aria-hidden="true"></span>{/if}
        {state.text}
      </span>
      {#if timing}<span class="timing num">{timing}</span>{/if}
    </div>
  </header>

  <div class="chart">
    {#if curve && status.heatingMode}
      <CurvePlot
        {curve}
        tint={tintOf(status.heatingMode)}
        {domain}
        {highlight}
        elapsedS={elapsed}
        animate
        height={196}
        minutesUnit={t.overview.curve.axisMinutes}
        label={t.overview.curve.chartLabel(modeName(status.heatingMode))}
      />
    {:else if status.heatingMode === "balanced" && nominalS && progress !== null}
      <div class="nominal" class:running={elapsed !== null}>
        <div
          class="track"
          role="progressbar"
          aria-label={t.overview.curve.progressLabel}
          aria-valuemin={0}
          aria-valuemax={nominalS}
          aria-valuenow={Math.round(progress * nominalS)}
          aria-valuetext={timing ?? undefined}
        >
          <div class="fill" style:transform="scaleX({progress})"></div>
          {#if elapsed !== null}
            <div class="head" style:left="{progress * 100}%"></div>
          {/if}
        </div>
        <div class="scale num" aria-hidden="true">
          <span>0:00</span>
          <span>{t.overview.curve.approxTotal(formatClock(nominalS))}</span>
        </div>
        <div class="note">
          <h3>{t.overview.curve.balancedTitle}</h3>
          <p>{t.overview.curve.balancedBody}</p>
        </div>
      </div>
    {:else}
      <div class="factory">
        <div class="factory-text"><p>{t.overview.curve.unknownMode}</p></div>
        <div class="factory-axis" aria-hidden="true"></div>
      </div>
    {/if}
  </div>
</section>

<style>
  .panel {
    padding: 16px 18px 12px;
    border-radius: var(--radius-group);
    background: var(--group);
    box-shadow: var(--surface-edge);
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 8px;
  }
  .title {
    min-width: 0;
  }
  h2 {
    font: var(--text-title);
  }
  .sub {
    margin-top: 2px;
    font: var(--text-callout);
    color: var(--label-2);
  }
  .state {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 3px;
    flex: none;
    text-align: right;
  }
  .state-text {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font: 500 13px/1.3 var(--font);
    color: var(--label-2);
  }
  .live .state-text {
    color: var(--label);
  }
  .warm .state-text {
    color: var(--orange);
  }
  .pip {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--tint);
  }
  .warm .pip {
    background: var(--orange);
  }
  .live .pip {
    animation: breathe 2.4s ease-in-out infinite;
  }
  @keyframes breathe {
    50% {
      opacity: 0.35;
    }
  }
  .timing {
    font: var(--text-callout);
    color: var(--label-2);
  }
  .chart {
    margin: 0 -2px;
  }
  .factory {
    position: relative;
    height: 196px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .factory-text {
    max-width: 46ch;
    text-align: center;
    padding-bottom: 24px;
  }
  .factory p {
    margin-top: 4px;
    font: var(--text-callout);
    color: var(--label-3);
  }
  .nominal {
    padding: 18px 0 4px;
  }
  .track {
    position: relative;
    height: 4px;
    margin: 0 2px;
    border-radius: 2px;
    background: var(--fill-strong);
  }
  .fill {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--tint);
    transform-origin: left center;
  }
  .head {
    position: absolute;
    top: 50%;
    width: 9px;
    height: 9px;
    margin: -4.5px 0 0 -4.5px;
    border-radius: 50%;
    background: var(--tint);
    box-shadow:
      0 0 0 2.5px var(--group),
      0 0 10px color-mix(in srgb, var(--tint) 60%, transparent);
  }
  .scale {
    display: flex;
    justify-content: space-between;
    margin: 7px 2px 0;
    font: var(--text-footnote);
    color: var(--label-3);
  }
  .running .scale {
    color: var(--label-2);
  }
  .note {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 0.5px solid var(--hairline);
  }
  .note h3 {
    font: var(--text-headline);
    color: var(--label-2);
  }
  .note p {
    max-width: 90ch;
    margin-top: 2px;
    font: var(--text-callout);
    color: var(--label-3);
  }
  .factory-axis {
    position: absolute;
    left: 1px;
    right: 42px;
    bottom: 24px;
    height: 1px;
    background: var(--hairline-strong);
  }
</style>
