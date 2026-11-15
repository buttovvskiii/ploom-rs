<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Group from "$lib/components/Group.svelte";
  import Row from "$lib/components/Row.svelte";
  import Switch from "$lib/components/Switch.svelte";
  import Icon from "$lib/components/icons/Icon.svelte";
  import { limitPhase, sticksLeft } from "$lib/features/limit/limit";
  import StickMarks from "$lib/features/limit/StickMarks.svelte";
  import { formatCountdown } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/index.svelte";
  import { run } from "$lib/state/api.svelte";
  import { clock } from "$lib/state/clock.svelte";
  import { prefs } from "$lib/state/prefs.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";

  const t = $derived(i18n.t);
  const status = $derived(snapshot.status);
  const busy = $derived(snapshot.busy);

  // Optimistic values while a command is in flight; the next snapshot settles them.
  let lockWanted = $state<boolean | null>(null);
  let autoWanted = $state<boolean | null>(null);
  let vibrationPending = $state(false);

  const locked = $derived(lockWanted ?? status.lock === "locked");
  const autoStart = $derived(autoWanted ?? status.autoStart === true);
  const lockOff = $derived(status.lock === "featureOff");

  const limit = $derived(snapshot.current.limit);
  const phase = $derived(limitPhase(limit, clock.now));
  const cooling = $derived(phase === "cooldown");
  const limitLine = $derived.by(() => {
    const o = t.overview;
    switch (phase) {
      case "cooldown":
        return o.limitLocked(formatCountdown(((limit.lockedUntilMs ?? 0) - clock.now) / 1000));
      case "pending":
        return o.limitPending;
      case "reached":
        return o.limitReached;
      default:
        return o.limitLeft(sticksLeft(limit), limit.settings.sticks);
    }
  });

  async function setLock(next: boolean): Promise<void> {
    lockWanted = next;
    await run((api) => api.setLock({ locked: next }));
    lockWanted = null;
  }

  async function setAutoStart(next: boolean): Promise<void> {
    autoWanted = next;
    await run((api) => api.setAutoStart({ enabled: next }));
    autoWanted = null;
  }

  async function toggleVibration(): Promise<void> {
    vibrationPending = true;
    await run((api) => api.setVibration({ on: !status.vibrating }));
    vibrationPending = false;
  }
</script>

<Group title={t.overview.controls}>
  {#if phase !== "off"}
    <button
      type="button"
      class="limit"
      onclick={() => prefs.go("limit")}
      aria-label="{t.limit.title}: {limitLine}. {t.overview.limitOpen}"
    >
      <span class="limit-icon"><Icon name="limit" size={16} /></span>
      <span class="limit-text">
        <span>{t.limit.title}</span>
        <span class="limit-line num" class:alert={phase === "cooldown" || phase === "reached"}
          >{limitLine}</span
        >
      </span>
      <StickMarks
        size="small"
        used={Math.min(limit.sticksUsed, limit.settings.sticks)}
        total={limit.settings.sticks}
        label={t.limit.usedLabel(
          Math.min(limit.sticksUsed, limit.settings.sticks),
          limit.settings.sticks,
        )}
        locked={phase === "cooldown" || phase === "pending"}
      />
      <Icon name="chevronRight" size={14} />
    </button>
  {/if}
  <Row
    label={t.overview.lockTitle}
    detail={cooling
      ? t.overview.lockCooldown
      : lockOff
        ? t.overview.lockUnavailable
        : t.overview.lockHint}
  >
    {#snippet leading()}<Icon name="lock" size={16} />{/snippet}
    <Switch
      label={t.overview.lockTitle}
      checked={locked}
      pending={lockWanted !== null}
      disabled={busy || lockOff || status.lock === null || (cooling && locked)}
      onchange={setLock}
    />
  </Row>
  <Row label={t.overview.autoStartTitle} detail={t.overview.autoStartHint}>
    {#snippet leading()}<Icon name="heating" size={16} />{/snippet}
    <Switch
      label={t.overview.autoStartTitle}
      checked={autoStart}
      pending={autoWanted !== null}
      disabled={busy || status.autoStart === null}
      onchange={setAutoStart}
    />
  </Row>
  <Row label={t.overview.findTitle} detail={t.overview.findHint}>
    {#snippet leading()}
      <span class="vibe" class:active={status.vibrating}><Icon name="vibrate" size={16} /></span>
    {/snippet}
    <Button size="small" onclick={toggleVibration} loading={vibrationPending} disabled={busy}>
      {status.vibrating ? t.overview.findStop : t.overview.findStart}
    </Button>
  </Row>
</Group>

<style>
  .limit {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 44px;
    padding: 8px var(--row-x);
    border: 0;
    border-radius: var(--radius-group) var(--radius-group) 0 0;
    background: transparent;
    color: var(--label-2);
    text-align: left;
    transition: background-color var(--dur-fast) ease;
  }
  .limit:hover {
    background: var(--group-hover);
  }
  .limit-icon {
    display: flex;
    flex: none;
  }
  .limit-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: var(--label);
  }
  .limit-line {
    font: var(--text-callout);
    color: var(--label-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .limit-line.alert {
    color: var(--orange);
  }

  .vibe {
    display: flex;
  }
  .vibe.active {
    color: var(--label);
    animation: buzz 0.18s linear infinite;
  }
  @keyframes buzz {
    25% {
      transform: translateX(-0.75px);
    }
    75% {
      transform: translateX(0.75px);
    }
  }
</style>
