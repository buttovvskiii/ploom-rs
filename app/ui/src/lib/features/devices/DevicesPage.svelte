<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Group from "$lib/components/Group.svelte";
  import Notice from "$lib/components/Notice.svelte";
  import Page from "$lib/components/Page.svelte";
  import Row from "$lib/components/Row.svelte";
  import SignalBars from "$lib/components/SignalBars.svelte";
  import Spinner from "$lib/components/Spinner.svelte";
  import Icon from "$lib/components/icons/Icon.svelte";
  import { i18n } from "$lib/i18n/index.svelte";
  import { run } from "$lib/state/api.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";
  import { scanner } from "./scan.svelte";

  const t = $derived(i18n.t);
  const snap = $derived(snapshot.current);
  const linked = $derived(snap.connection !== "idle");
  const working = $derived(snap.connection === "connecting" || snap.connection === "reconnecting");
  const connectedId = $derived(snap.connection === "connected" ? (snap.device?.id ?? null) : null);

  let pendingId = $state<string | null>(null);

  async function connect(id: string | null): Promise<void> {
    pendingId = id ?? "nearest";
    await run((api) => api.connect({ id }));
    pendingId = null;
  }

  async function disconnect(): Promise<void> {
    await run((api) => api.disconnect());
  }

  const signalText = (dbm: number | null) =>
    dbm === null ? t.devices.noSignal : t.devices.signal(dbm);
</script>

<Page title={t.devices.title}>
  {#snippet actions()}
    <Button
      size="small"
      onclick={() => scanner.scan()}
      loading={scanner.pending || snap.scanning}
      disabled={snap.connectionError === "bluetoothUnavailable"}
    >
      {scanner.pending || snap.scanning ? t.devices.scanning : t.devices.scan}
    </Button>
  {/snippet}

  <Group title={t.devices.current}>
    {#if linked}
      <Row>
        {#snippet leading()}
          {#if working}<Spinner size={16} />{:else}<Icon name="device" size={18} />{/if}
        {/snippet}
        {#snippet content()}
          <span class="name">{snap.device?.name ?? t.shell.noDevice}</span>
          <span class="state" class:ok={snap.connection === "connected"}>
            {t.connection[snap.connection]}
            {#if snap.connectionError && working}· {t.errors[snap.connectionError]}{/if}
          </span>
        {/snippet}
        <Button size="small" onclick={disconnect}>{t.devices.disconnect}</Button>
      </Row>
      {#if snap.device}
        <Row label={t.devices.identifier}>
          <span class="id selectable">{snap.device.id}</span>
        </Row>
        <Row label={t.devices.hardware}><span>{snap.device.hardware ?? t.common.dash}</span></Row>
        <Row label={t.devices.firmware}><span>{snap.device.firmware ?? t.common.dash}</span></Row>
        <Row label={t.overview.signal}>
          <SignalBars rssi={snap.status.rssiDbm} />
          <span class="num"
            >{snap.status.rssiDbm === null
              ? t.common.dash
              : t.devices.signal(snap.status.rssiDbm)}</span
          >
        </Row>
      {/if}
    {:else}
      <Row
        label={t.devices.none}
        detail={snap.connectionError ? t.errors[snap.connectionError] : t.disconnected.hintWake}
      >
        {#snippet leading()}<Icon name="device" size={18} />{/snippet}
        <Button
          size="small"
          variant="primary"
          loading={pendingId === "nearest"}
          disabled={snap.connectionError === "bluetoothUnavailable"}
          onclick={() => connect(null)}
        >
          {t.devices.connectFirst}
        </Button>
      </Row>
    {/if}
  </Group>

  <Group title={t.devices.nearby}>
    {#if scanner.results === null}
      <p class="empty">{scanner.pending ? t.devices.scanning : t.devices.notScanned}</p>
    {:else if scanner.results.length === 0}
      <p class="empty">{t.devices.nothingFound} {t.disconnected.hintWake}</p>
    {:else}
      {#each scanner.results as device (device.id)}
        {@const current = device.id === connectedId}
        <Row>
          {#snippet leading()}
            <SignalBars rssi={device.rssiDbm} label={signalText(device.rssiDbm)} />
          {/snippet}
          {#snippet content()}
            <span class="name">{device.name}</span>
            <span class="meta num">
              {signalText(device.rssiDbm)} · <span class="id">{device.id}</span>
            </span>
          {/snippet}
          {#if current}
            <span class="badge"><Icon name="check" size={14} />{t.devices.thisDevice}</span>
          {:else}
            <Button
              size="small"
              loading={pendingId === device.id}
              disabled={snapshot.busy}
              onclick={() => connect(device.id)}
            >
              {connectedId ? t.devices.switch : t.devices.connect}
            </Button>
          {/if}
        </Row>
      {/each}
    {/if}
  </Group>

  <Notice>{t.devices.warning}</Notice>
</Page>

<style>
  .name {
    color: var(--label);
  }
  .state {
    font: var(--text-callout);
    color: var(--label-2);
  }
  .meta {
    font: var(--text-callout);
    color: var(--label-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .id {
    font: var(--text-callout);
    color: var(--label-3);
    -webkit-user-select: text;
    user-select: text;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font: var(--text-callout);
    color: var(--label-2);
  }
  .empty {
    padding: 14px var(--row-x);
    font: var(--text-callout);
    color: var(--label-2);
  }
</style>
