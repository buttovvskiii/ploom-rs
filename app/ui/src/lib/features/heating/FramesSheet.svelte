<script lang="ts">
  import type { FramePreview, ProfileTarget } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import Sheet from "$lib/components/Sheet.svelte";
  import Spinner from "$lib/components/Spinner.svelte";
  import { formatOpcode, spacedHex } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/index.svelte";
  import { targetLabel } from "$lib/modes";
  import { run } from "$lib/state/api.svelte";

  interface Props {
    target: ProfileTarget | null;
    onclose: () => void;
  }
  let { target, onclose }: Props = $props();

  const t = $derived(i18n.t);
  let preview = $state<FramePreview | null>(null);
  let label = $state("");
  let copied = $state(false);

  $effect(() => {
    if (!target) return;
    const requested = target;
    label = targetLabel(requested);
    preview = null;
    copied = false;
    let cancelled = false;
    void run((api) => api.previewProfile({ target: requested })).then((result) => {
      if (cancelled) return;
      if (result) preview = result;
      else onclose();
    });
    return () => {
      cancelled = true;
    };
  });

  async function copy(): Promise<void> {
    if (!preview) return;
    try {
      await navigator.clipboard.writeText(preview.frames.map((f) => spacedHex(f.hex)).join("\n"));
      copied = true;
    } catch {
      copied = false;
    }
  }
</script>

<Sheet open={target !== null} title={t.heating.frames.title} {onclose} width={600}>
  <p class="subtitle">{label}</p>
  {#if !preview}
    <div class="loading"><Spinner size={18} /></div>
  {:else}
    <p class="meta">
      {t.heating.frames.subtitle(preview.frames.length)} ·
      {preview.calibrationKey === null
        ? t.heating.frames.noCalibration
        : t.heating.frames.calibration(preview.calibrationKey)}
    </p>
    <ol class="frames selectable">
      {#each preview.frames as frame, i (i)}
        <li>
          <span class="index">{i + 1}</span>
          <code>{spacedHex(frame.hex)}</code>
          {#if frame.awaitsOpcode !== null}
            <span class="awaits">{t.heating.frames.awaits(formatOpcode(frame.awaitsOpcode))}</span>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}

  {#snippet footer()}
    <Button onclick={copy} disabled={!preview}>{copied ? t.common.copied : t.common.copy}</Button>
    <Button variant="primary" onclick={onclose} data-autofocus>{t.common.done}</Button>
  {/snippet}
</Sheet>

<style>
  .subtitle {
    color: var(--label);
  }
  .meta {
    margin: 2px 0 12px;
    font: var(--text-callout);
    color: var(--label-2);
  }
  .loading {
    display: grid;
    place-items: center;
    height: 160px;
    color: var(--label-2);
  }
  .frames {
    list-style: none;
    margin: 0;
    padding: 6px 0;
    max-height: 46vh;
    overflow: auto;
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.22);
    box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.06);
  }
  li {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 2px 12px;
    font: 400 11.5px/1.6 var(--mono);
  }
  .index {
    width: 2ch;
    flex: none;
    text-align: right;
    color: var(--label-3);
    font-variant-numeric: tabular-nums;
  }
  code {
    font: inherit;
    color: var(--label);
    white-space: nowrap;
  }
  .awaits {
    margin-left: auto;
    font: var(--text-footnote);
    color: var(--label-2);
    white-space: nowrap;
  }
</style>
