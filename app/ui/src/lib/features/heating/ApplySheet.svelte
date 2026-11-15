<script lang="ts">
  import type { ProfileTarget } from "$lib/api";
  import { ApiError } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import CurvePlot from "$lib/components/CurvePlot.svelte";
  import ProgressBar from "$lib/components/ProgressBar.svelte";
  import Sheet from "$lib/components/Sheet.svelte";
  import Icon from "$lib/components/icons/Icon.svelte";
  import { customCurve, domainFor, stepRange } from "$lib/curve/curve";
  import { i18n } from "$lib/i18n/index.svelte";
  import { modeName, targetLabel, tintOf } from "$lib/modes";
  import { run } from "$lib/state/api.svelte";
  import { catalog } from "$lib/state/catalog.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";

  interface Props {
    target: ProfileTarget | null;
    onclose: () => void;
  }
  let { target, onclose }: Props = $props();

  type Phase = "confirm" | "writing" | "done" | "failed";
  let phase = $state<Phase>("confirm");
  let failure = $state<string | null>(null);
  let shown = $state<ProfileTarget | null>(null);

  // Keep the last target while the sheet animates out.
  $effect(() => {
    if (target) {
      shown = target;
      phase = "confirm";
      failure = null;
    }
  });

  const t = $derived(i18n.t);
  const base = $derived(shown ? (shown.kind === "preset" ? shown.mode : shown.base) : null);
  const mode = $derived(catalog.mode(base));
  const curve = $derived.by(() => {
    if (!shown || !mode) return null;
    return shown.kind === "custom"
      ? customCurve(mode, shown.holdTempC, shown.holdSecs)
      : mode.curve;
  });
  const domain = $derived(
    domainFor((catalog.current?.modes ?? []).flatMap((m) => (m.curve ? [m.curve] : []))),
  );
  const highlight = $derived(
    shown?.kind === "custom" && curve && mode?.holdStepIndex != null
      ? stepRange(curve, mode.holdStepIndex)
      : null,
  );

  const op = $derived(snapshot.current.operation);
  const title = $derived(
    !shown
      ? ""
      : phase === "done"
        ? t.heating.sheet.success
        : phase === "failed"
          ? t.heating.sheet.failed
          : shown.kind === "custom"
            ? t.heating.sheet.titleCustom
            : t.heating.sheet.title(modeName(shown.mode)),
  );

  async function apply(): Promise<void> {
    if (!shown) return;
    const requested = shown;
    phase = "writing";
    try {
      await run((api) => api.applyProfile({ target: requested }), { rethrow: true });
      phase = "done";
      setTimeout(() => {
        if (phase === "done") onclose();
      }, 1600);
    } catch (error) {
      const e = ApiError.from(error);
      failure = t.errors[e.code];
      phase = "failed";
    }
  }
</script>

<Sheet open={target !== null} {title} {onclose} dismissable={phase !== "writing"}>
  {#if shown && base}
    <div class="preview" style:--tint={tintOf(base)}>
      {#if curve}
        <CurvePlot
          {curve}
          tint={tintOf(base)}
          {domain}
          {highlight}
          height={120}
          minutesUnit={t.overview.curve.axisMinutes}
          label={t.overview.curve.chartLabel(targetLabel(shown))}
        />
      {/if}
    </div>

    {#if phase === "confirm"}
      <p>{t.heating.sheet.body}</p>
      <p class="quiet">{t.heating.sheet.fallback}</p>
    {:else if phase === "writing"}
      <div class="progress">
        <ProgressBar
          value={op?.done ?? 0}
          max={op?.total ?? 38}
          tint={tintOf(base)}
          label={t.heating.sheet.progress(op?.done ?? 0, op?.total ?? 38)}
        />
        <p class="num" aria-live="polite">
          {op ? t.heating.sheet.progress(op.done, op.total) : t.heating.sheet.preparing}
        </p>
      </div>
    {:else if phase === "done"}
      <p class="result" role="status">
        <Icon name="check" size={16} />
        {t.heating.sheet.successBody(targetLabel(shown))}
      </p>
    {:else}
      <p class="result failed" role="alert">
        <Icon name="warning" size={16} />
        {failure}
      </p>
      <p class="quiet">{t.heating.sheet.fallback}</p>
    {/if}
  {/if}

  {#snippet footer()}
    {#if phase === "confirm"}
      <Button onclick={onclose}>{t.common.cancel}</Button>
      <Button variant="primary" onclick={apply} data-autofocus>{t.heating.sheet.confirm}</Button>
    {:else if phase === "writing"}
      <Button disabled>{t.common.cancel}</Button>
    {:else if phase === "done"}
      <Button variant="primary" onclick={onclose} data-autofocus>{t.common.done}</Button>
    {:else}
      <Button onclick={onclose}>{t.common.close}</Button>
      <Button variant="primary" onclick={apply} data-autofocus>{t.common.tryAgain}</Button>
    {/if}
  {/snippet}
</Sheet>

<style>
  .preview {
    margin: 6px 0 14px;
  }
  p + p {
    margin-top: 6px;
  }
  .quiet {
    font: var(--text-callout);
    color: var(--label-3);
  }
  .progress {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .progress p {
    font: var(--text-callout);
  }
  .result {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--label);
  }
  .result :global(.icon) {
    color: var(--green);
  }
  .failed :global(.icon) {
    color: var(--orange);
  }
</style>
