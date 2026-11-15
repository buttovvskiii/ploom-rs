<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Icon from "$lib/components/icons/Icon.svelte";
  import Spinner from "$lib/components/Spinner.svelte";
  import { i18n } from "$lib/i18n/index.svelte";
  import { run } from "$lib/state/api.svelte";
  import { prefs } from "$lib/state/prefs.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";

  interface Props {
    /** Inline variant for sections that need a device; full one for Overview. */
    compact?: boolean;
    /** Beside the 3D device on Overview: left-aligned, the device is the picture. */
    hero?: boolean;
  }
  let { compact = false, hero = false }: Props = $props();

  const t = $derived(i18n.t);
  const snap = $derived(snapshot.current);
  const noBluetooth = $derived(snap.connectionError === "bluetoothUnavailable");
  const working = $derived(snap.connection === "connecting" || snap.connection === "reconnecting");

  const copy = $derived.by(() => {
    const d = t.disconnected;
    if (noBluetooth) return { title: d.bluetoothTitle, body: d.bluetoothBody };
    if (snap.connection === "connecting")
      return { title: d.connectingTitle, body: d.connectingBody };
    if (snap.connection === "reconnecting") {
      return { title: d.reconnectingTitle, body: d.reconnectingBody };
    }
    return { title: d.idleTitle, body: d.idleBody };
  });
  const errorLine = $derived(
    snap.connectionError && !noBluetooth ? t.errors[snap.connectionError] : null,
  );

  let starting = $state(false);
  async function connect(): Promise<void> {
    starting = true;
    await run((api) => api.connect({ id: null }));
    starting = false;
  }
</script>

{#if compact}
  <div class="compact">
    <Icon name="device" size={18} />
    <p>{working ? copy.title : t.disconnected.needsConnection}</p>
    {#if !working && !noBluetooth}
      <Button size="small" onclick={connect} loading={starting}>{t.disconnected.connect}</Button>
    {/if}
  </div>
{:else}
  <div class="state" class:hero>
    {#if !hero}
      <div class="mark" aria-hidden="true">
        {#if working}
          <Spinner size={22} />
        {:else}
          <Icon name="device" size={34} strokeWidth={1.2} />
        {/if}
      </div>
    {/if}
    <h2>
      {#if hero && working}<span class="inline-spinner"><Spinner size={14} /></span>{/if}
      {copy.title}
    </h2>
    <p class="body">{copy.body}</p>
    {#if errorLine}<p class="error">{errorLine}</p>{/if}

    {#if !noBluetooth}
      <ul class="hints">
        <li>{t.disconnected.hintWake}</li>
        <li>{t.disconnected.hintExclusive}</li>
      </ul>
      {#if !working}
        <div class="buttons">
          <Button variant="primary" onclick={connect} loading={starting}>
            {t.disconnected.connect}
          </Button>
          <Button onclick={() => prefs.go("devices")}>{t.disconnected.chooseDevice}</Button>
        </div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 64px 24px 24px;
    max-width: 440px;
    margin: 0 auto;
  }
  .state.hero {
    flex: 1;
    min-width: 0;
    align-items: flex-start;
    text-align: left;
    padding: 0;
    margin: 0;
  }
  .hero h2 {
    display: flex;
    align-items: center;
    gap: 9px;
    font: var(--text-display);
    letter-spacing: -0.01em;
  }
  .hero .hints {
    margin-top: 18px;
    align-self: stretch;
  }
  .inline-spinner {
    display: flex;
    color: var(--label-2);
  }
  .mark {
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    margin-bottom: 18px;
    border-radius: 16px;
    background: var(--group);
    color: var(--label-2);
    box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.06);
  }
  h2 {
    font: var(--text-title);
  }
  .body {
    margin-top: 6px;
    color: var(--label-2);
  }
  .error {
    margin-top: 6px;
    font: var(--text-callout);
    color: var(--orange);
  }
  .hints {
    margin: 22px 0 0;
    padding: 14px 16px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-radius: var(--radius-group);
    background: var(--group);
    color: var(--label-2);
    font: var(--text-callout);
    text-align: left;
  }
  .hints li {
    position: relative;
    padding-left: 14px;
  }
  .hints li::before {
    content: "";
    position: absolute;
    left: 2px;
    top: 0.62em;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--label-3);
  }
  .buttons {
    display: flex;
    gap: 8px;
    margin-top: 20px;
  }
  .compact {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px var(--row-x);
    border-radius: var(--radius-group);
    background: var(--group);
    color: var(--label-2);
  }
  .compact p {
    flex: 1;
  }
</style>
