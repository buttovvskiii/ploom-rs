<script lang="ts">
  import type { ModeInfo } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import CurvePlot from "$lib/components/CurvePlot.svelte";
  import HeatDots from "$lib/components/HeatDots.svelte";
  import Icon from "$lib/components/icons/Icon.svelte";
  import { i18n } from "$lib/i18n/index.svelte";
  import { modeName, tintOf } from "$lib/modes";

  interface Props {
    mode: ModeInfo;
    domain: [number, number];
    active: boolean;
    activeCustom: boolean;
    disabled: boolean;
    loading?: boolean;
    onapply: () => void;
  }
  let { mode, domain, active, activeCustom, disabled, loading = false, onapply }: Props = $props();

  const t = $derived(i18n.t);
  const tint = $derived(tintOf(mode.id));
</script>

<div class="mode" class:active>
  <div class="thumb" aria-hidden="true">
    {#if mode.curve}
      <CurvePlot curve={mode.curve} {tint} {domain} variant="thumb" height={30} label="" />
    {:else}
      <svg width="64" height="30" viewBox="0 0 64 30">
        <line x1="1" x2="63" y1="27" y2="27" class="base" />
        <line x1="1" x2="63" y1="12" y2="12" class="unknown" style:stroke={tint} />
      </svg>
    {/if}
  </div>

  <div class="text">
    <div class="name-line">
      <span class="name">{modeName(mode.id)}</span>
      <HeatDots heat={mode.heat} {tint} label={t.heating.intensity(mode.heat)} />
    </div>
    <span class="detail"
      >{t.heating.sticks(mode.maxSticks)} · {t.heating.minutes(mode.minutes)}</span
    >
  </div>

  <div class="trailing">
    {#if active}
      <span class="badge">
        <Icon name="check" size={14} />
        {activeCustom ? t.heating.activeCustom : t.heating.active}
      </span>
    {:else}
      <Button size="small" {disabled} {loading} onclick={onapply}>
        {mode.id === "balanced" ? t.heating.applyBalanced : t.heating.apply}
      </Button>
    {/if}
  </div>
</div>

<style>
  .mode {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 58px;
    padding: 8px var(--row-x);
  }
  .mode:not(:first-child)::before {
    content: "";
    position: absolute;
    top: 0;
    left: calc(var(--row-x) + 78px);
    right: 0;
    height: 0.5px;
    background: var(--hairline-strong);
  }
  .thumb {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 64px;
    height: 30px;
    flex: none;
  }
  .base {
    stroke: rgba(255, 255, 255, 0.1);
    stroke-width: 1;
  }
  .unknown {
    stroke-width: 1.3;
    stroke-dasharray: 1.5 3.5;
    stroke-linecap: round;
    opacity: 0.85;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .name-line {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .name {
    font: 500 13px/1.3 var(--font);
  }
  .detail {
    font: var(--text-callout);
    color: var(--label-2);
  }
  .trailing {
    flex: none;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font: var(--text-callout);
    color: var(--label-2);
  }
</style>
