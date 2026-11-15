<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Group from "$lib/components/Group.svelte";
  import Notice from "$lib/components/Notice.svelte";
  import Page from "$lib/components/Page.svelte";
  import Row from "$lib/components/Row.svelte";
  import Spinner from "$lib/components/Spinner.svelte";
  import DisconnectedState from "$lib/features/shell/DisconnectedState.svelte";
  import { formatClock, formatDuration, formatNumber } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/index.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";
  import { logs } from "./logs.svelte";
  import PuffTimeline from "./PuffTimeline.svelte";

  const t = $derived(i18n.t);
  const report = $derived(logs.report);
  const onOff = (v: boolean) => (v ? t.common.on : t.common.off);
  const volts = (mv: number) => formatNumber(mv / 1000, 2);

  $effect(() => {
    if (snapshot.connected && !logs.report && !logs.loading && !snapshot.busy) void logs.read();
  });
</script>

<Page title={t.history.title}>
  {#snippet actions()}
    <Button
      size="small"
      onclick={() => logs.read()}
      loading={logs.loading}
      disabled={!snapshot.connected || snapshot.busy}
    >
      {report ? t.common.reload : t.history.read}
    </Button>
  {/snippet}

  {#if !snapshot.connected}
    <DisconnectedState compact />
  {/if}

  {#if !report}
    {#if logs.loading}
      <div class="loading"><Spinner size={18} /><span>{t.history.reading}</span></div>
    {:else if snapshot.connected}
      <p class="quiet">{t.history.notRead}</p>
    {/if}
  {:else}
    <Group title={t.history.session}>
      {#if report.session}
        {@const s = report.session}
        <div class="figures">
          <div>
            <span class="caption">{t.history.duration}</span>
            <span class="figure num">{formatClock(s.durationS)}</span>
          </div>
          <div>
            <span class="caption">{t.history.puffs}</span>
            <span class="figure num">{s.puffCount}</span>
          </div>
        </div>
        <div class="timeline">
          <PuffTimeline
            durationS={s.durationS}
            offsetsS={s.puffOffsetsS}
            minutesUnit={t.overview.curve.axisMinutes}
            label={t.history.timelineLabel(s.puffCount, formatDuration(s.durationS))}
          />
        </div>
        <Row label={t.history.puffStop}><span>{onOff(s.puffStopEnabled)}</span></Row>
        <Row label={t.history.autoStart}><span>{onOff(s.autoStartEnabled)}</span></Row>
        <Row label={t.history.lock}><span>{onOff(s.lockEnabled)}</span></Row>
        <Row label={t.history.counter}>
          <span class="num selectable">{formatNumber(s.startCounter)}</span>
        </Row>
      {:else}
        <p class="empty">{t.history.sessionEmpty}</p>
      {/if}
    </Group>

    <Group title={t.history.charge}>
      {#if report.charge}
        {@const c = report.charge}
        <Row label={t.history.voltage}>
          <span class="num">{t.history.voltageRange(volts(c.startMv), volts(c.endMv))}</span>
        </Row>
        <Row label={t.history.chargeCount}>
          <span class="num">{c.totalChargeCount ?? t.common.dash}</span>
        </Row>
        <Row label={t.history.soh}>
          <span class="num"
            >{c.stateOfHealthPct === null ? t.common.dash : `${c.stateOfHealthPct}%`}</span
          >
        </Row>
        <Row label={t.history.counter}>
          <span class="num selectable">
            {t.history.counterRange(formatNumber(c.startCounter), formatNumber(c.endCounter))}
          </span>
        </Row>
      {:else}
        <p class="empty">{t.history.chargeEmpty}</p>
      {/if}
    </Group>

    {#if report.unacquired}
      <Group>
        <Row label={t.history.queue}>
          <span class="num">
            {t.history.queueValue(report.unacquired.sessions, report.unacquired.charges)}
          </span>
        </Row>
      </Group>
    {/if}
  {/if}

  <Notice>{t.history.note}</Notice>
</Page>

<style>
  .figures {
    display: flex;
    gap: 40px;
    padding: 14px var(--row-x) 4px;
  }
  .figures div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .caption {
    font: var(--text-callout);
    color: var(--label-2);
  }
  .figure {
    font: 300 30px/1.1 var(--font);
    letter-spacing: -0.01em;
  }
  .timeline {
    padding: 4px var(--row-x) 10px;
  }
  .empty,
  .quiet {
    padding: 14px var(--row-x);
    font: var(--text-callout);
    color: var(--label-2);
  }
  .quiet {
    padding: 0 4px;
  }
  .loading {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 4px;
    color: var(--label-2);
  }
</style>
