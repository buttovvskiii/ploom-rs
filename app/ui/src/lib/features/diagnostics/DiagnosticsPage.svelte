<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Group from "$lib/components/Group.svelte";
  import Page from "$lib/components/Page.svelte";
  import Row from "$lib/components/Row.svelte";
  import Spinner from "$lib/components/Spinner.svelte";
  import DisconnectedState from "$lib/features/shell/DisconnectedState.svelte";
  import { formatNumber, formatTemp } from "$lib/i18n/format";
  import { i18n, lookup } from "$lib/i18n/index.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";
  import BarList from "./BarList.svelte";
  import { diagnostics } from "./diagnostics.svelte";

  const t = $derived(i18n.t);
  const d = $derived(diagnostics.report);
  const dash = $derived(t.common.dash);

  const starts = $derived(
    (d?.heatingStarts ?? []).map((s) => ({
      key: s.key,
      label: lookup(t.diagnostics.startKeys, s.key),
      value: s.count,
      display: formatNumber(s.count),
    })),
  );
  const startsTotal = $derived((d?.heatingStarts ?? []).reduce((sum, s) => sum + s.count, 0));

  // CC lines on a fixed scale so a pulled-up line reads as clearly "high".
  const cc = $derived(
    d?.usbCc
      ? d.usbCc.map((value, i) => ({
          key: `cc${i + 1}`,
          label: `CC${i + 1}`,
          value,
          display: formatNumber(value),
        }))
      : [],
  );

  $effect(() => {
    if (snapshot.connected && !diagnostics.report && !diagnostics.loading && !snapshot.busy) {
      void diagnostics.read();
    }
  });
</script>

<Page title={t.diagnostics.title}>
  {#snippet actions()}
    <Button
      size="small"
      onclick={() => diagnostics.read()}
      loading={diagnostics.loading}
      disabled={!snapshot.connected || snapshot.busy}
    >
      {t.common.reload}
    </Button>
  {/snippet}

  {#if !snapshot.connected}
    <DisconnectedState compact />
  {/if}

  {#if !d}
    {#if diagnostics.loading}
      <div class="loading"><Spinner size={18} /><span>{t.diagnostics.reading}</span></div>
    {/if}
  {:else}
    <Group title={t.diagnostics.power}>
      <Row label={t.diagnostics.battery}>
        <span class="num">{d.batteryPct === null ? dash : `${d.batteryPct}%`}</span>
      </Row>
      <Row label={t.diagnostics.health}>
        <span class="num">{d.batteryHealthPct === null ? dash : `${d.batteryHealthPct}%`}</span>
      </Row>
      <Row label={t.diagnostics.cycles}>
        <span class="num">{d.chargeCycles === null ? dash : formatNumber(d.chargeCycles)}</span>
      </Row>
      <Row label={t.diagnostics.usb}>
        <span>
          {d.usbPlugged === null
            ? dash
            : d.usbPlugged
              ? t.diagnostics.usbPlugged
              : t.diagnostics.usbUnplugged}
        </span>
      </Row>
      {#if cc.length}
        <Row label={t.diagnostics.cc} detail={t.diagnostics.ccHint} stacked>
          <div class="cc"><BarList items={cc} max={600} /></div>
        </Row>
      {/if}
    </Group>

    <Group title={t.diagnostics.thermal}>
      <Row label={t.diagnostics.enclosure}>
        <span class="num">
          {d.enclosureTempC === null
            ? snapshot.inSession
              ? t.diagnostics.enclosureHeating
              : dash
            : formatTemp(d.enclosureTempC, 2)}
        </span>
      </Row>
    </Group>

    {#if starts.length}
      <Group
        title={t.diagnostics.starts}
        footer={t.diagnostics.startsTotal(formatNumber(startsTotal))}
      >
        <BarList items={starts} />
      </Group>
    {/if}

    {#if d.adc}
      <Group title={t.diagnostics.adc} footer={t.diagnostics.adcFooter}>
        <table>
          <thead class="visually-hidden">
            <tr
              ><th scope="col">{t.diagnostics.name}</th><th scope="col">{t.diagnostics.value}</th
              ></tr
            >
          </thead>
          <tbody>
            {#each d.adc as channel (channel.name)}
              <tr>
                <th scope="row">
                  {lookup(t.diagnostics.adcKeys, channel.name)}
                  <span class="raw">{channel.name}</span>
                </th>
                <td class="num selectable">{formatNumber(channel.value)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </Group>
    {/if}
  {/if}
</Page>

<style>
  .loading {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 4px;
    color: var(--label-2);
  }
  .cc {
    width: 100%;
    margin: -6px calc(-1 * var(--row-x)) -8px;
    width: calc(100% + 2 * var(--row-x));
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  tr + tr th,
  tr + tr td {
    border-top: 0.5px solid var(--hairline-strong);
  }
  tr + tr th {
    border-image: linear-gradient(
        to right,
        transparent var(--row-x),
        var(--hairline-strong) var(--row-x)
      )
      1;
  }
  th,
  td {
    height: 36px;
    padding: 0 var(--row-x);
    font: var(--text-body);
  }
  th {
    text-align: left;
    font-weight: 400;
  }
  .raw {
    margin-left: 8px;
    font: var(--text-footnote);
    color: var(--label-3);
  }
  td {
    text-align: right;
    color: var(--label-2);
  }
</style>
