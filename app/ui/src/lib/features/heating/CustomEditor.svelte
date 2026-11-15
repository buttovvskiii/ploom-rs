<script lang="ts">
  import type { HeatingMode, ProfileTarget } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import CurvePlot from "$lib/components/CurvePlot.svelte";
  import Group from "$lib/components/Group.svelte";
  import Row from "$lib/components/Row.svelte";
  import SegmentedControl from "$lib/components/SegmentedControl.svelte";
  import Slider from "$lib/components/Slider.svelte";
  import { customCurve, domainFor, stepRange, totalDuration } from "$lib/curve/curve";
  import { formatDuration } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/index.svelte";
  import { modeName, tintOf } from "$lib/modes";
  import { catalog } from "$lib/state/catalog.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";

  interface Props {
    disabled: boolean;
    onapply: (target: ProfileTarget) => void;
    onpreview: (target: ProfileTarget) => void;
  }
  let { disabled, onapply, onpreview }: Props = $props();

  const t = $derived(i18n.t);
  const limits = $derived(catalog.current?.custom ?? null);

  function holdOf(mode: HeatingMode): { temp: number; secs: number } {
    const info = catalog.mode(mode);
    const step = info?.curve && info.holdStepIndex !== null ? info.curve[info.holdStepIndex] : null;
    return { temp: Math.abs(step?.temperatureC ?? 260), secs: step?.durationS ?? 120 };
  }

  // Start from the active custom profile if there is one, else the first base's own hold.
  const initial = snapshot.status.customProfile;
  const firstBase: HeatingMode = initial?.base ?? catalog.current?.custom.bases[0] ?? "delicate";
  let base = $state<HeatingMode>(firstBase);
  let holdTemp = $state(initial?.holdTempC ?? holdOf(firstBase).temp);
  let holdSecs = $state(initial?.holdSecs ?? holdOf(firstBase).secs);

  // When the device reports a (different) custom profile, adopt it as the starting point.
  const activeKey = $derived.by(() => {
    const c = snapshot.status.customProfile;
    return c ? `${c.base}:${c.holdTempC}:${c.holdSecs}` : null;
  });
  let adopted: string | null = initial
    ? `${initial.base}:${initial.holdTempC}:${initial.holdSecs}`
    : null;
  $effect(() => {
    const c = snapshot.status.customProfile;
    if (!activeKey || !c || activeKey === adopted) return;
    adopted = activeKey;
    base = c.base;
    holdTemp = c.holdTempC;
    holdSecs = c.holdSecs;
  });

  function changeBase(next: HeatingMode): void {
    const hold = holdOf(next);
    holdTemp = hold.temp;
    holdSecs = hold.secs;
  }

  const mode = $derived(catalog.mode(base));
  const curve = $derived(mode ? customCurve(mode, holdTemp, holdSecs) : null);
  const domain = $derived(
    domainFor((catalog.current?.modes ?? []).flatMap((m) => (m.curve ? [m.curve] : []))),
  );
  const highlight = $derived(
    curve && mode?.holdStepIndex != null ? stepRange(curve, mode.holdStepIndex) : null,
  );
  const preset = $derived(holdOf(base));
  const unchanged = $derived(holdTemp === preset.temp && holdSecs === preset.secs);

  const target = $derived<ProfileTarget>({ kind: "custom", base, holdTempC: holdTemp, holdSecs });
  const active = $derived.by(() => {
    const c = snapshot.status.customProfile;
    return c !== null && c.base === base && c.holdTempC === holdTemp && c.holdSecs === holdSecs;
  });
</script>

{#if limits && mode}
  <Group title={t.heating.custom.title} footer={t.heating.custom.footer}>
    <Row label={t.heating.custom.base} stacked>
      <div class="base">
        <SegmentedControl
          label={t.heating.custom.base}
          options={limits.bases.map((b) => ({ value: b, label: modeName(b) }))}
          bind:value={base}
          onchange={changeBase}
        />
      </div>
    </Row>

    <Row stacked>
      {#snippet content()}
        <div class="slider-head">
          <span>{t.heating.custom.holdTemp}</span>
          <span class="value num">{holdTemp} °C</span>
        </div>
      {/snippet}
      <Slider
        bind:value={holdTemp}
        min={limits.tempC[0]}
        max={limits.tempC[1]}
        label={t.heating.custom.holdTemp}
        valueText="{holdTemp} °C"
        tint={tintOf(base)}
      />
      <div class="limits num" aria-hidden="true">
        <span>{limits.tempC[0]} °C</span><span>{limits.tempC[1]} °C</span>
      </div>
    </Row>

    <Row stacked>
      {#snippet content()}
        <div class="slider-head">
          <span>{t.heating.custom.holdDuration}</span>
          <span class="value num">{formatDuration(holdSecs)}</span>
        </div>
      {/snippet}
      <Slider
        bind:value={holdSecs}
        min={limits.holdSecs[0]}
        max={limits.holdSecs[1]}
        label={t.heating.custom.holdDuration}
        valueText={formatDuration(holdSecs)}
        tint={tintOf(base)}
      />
      <div class="limits num" aria-hidden="true">
        <span>{formatDuration(limits.holdSecs[0])}</span>
        <span>{formatDuration(limits.holdSecs[1])}</span>
      </div>
    </Row>

    <div class="preview">
      <div class="preview-head">
        <span>{t.heating.custom.preview}</span>
        <span class="num">
          {unchanged ? t.heating.custom.unchanged : ""}
          {#if curve}{unchanged ? " · " : ""}{t.heating.custom.total(
              formatDuration(totalDuration(curve)),
            )}{/if}
        </span>
      </div>
      {#if curve}
        <CurvePlot
          {curve}
          ghost={unchanged ? null : mode.curve}
          tint={tintOf(base)}
          {domain}
          {highlight}
          height={150}
          minutesUnit={t.overview.curve.axisMinutes}
          label={t.heating.custom.previewLabel}
        />
      {/if}
      <div class="buttons">
        <Button onclick={() => onpreview(target)} disabled={!snapshot.connected}>
          {t.heating.custom.showFrames}
        </Button>
        <Button variant="primary" onclick={() => onapply(target)} disabled={disabled || active}>
          {active ? t.heating.active : t.heating.custom.apply}
        </Button>
      </div>
    </div>
  </Group>
{/if}

<style>
  .base {
    width: 100%;
  }
  .slider-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  .value {
    color: var(--label);
    font-weight: 500;
  }
  .limits {
    display: flex;
    justify-content: space-between;
    width: 100%;
    margin-top: -2px;
    font: var(--text-footnote);
    color: var(--label-3);
  }
  .preview {
    position: relative;
    padding: 12px var(--row-x) 14px;
  }
  .preview::before {
    content: "";
    position: absolute;
    top: 0;
    left: var(--row-x);
    right: 0;
    height: 0.5px;
    background: var(--hairline-strong);
  }
  .preview-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 6px;
    font: var(--text-callout);
    color: var(--label-2);
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 12px;
  }
</style>
