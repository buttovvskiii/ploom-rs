<script lang="ts">
  import type { LimitSettings } from "$lib/api";
  import Button from "$lib/components/Button.svelte";
  import ChoiceChips from "$lib/components/ChoiceChips.svelte";
  import Disclosure from "$lib/components/Disclosure.svelte";
  import Icon from "$lib/components/icons/Icon.svelte";
  import Page from "$lib/components/Page.svelte";
  import Stepper from "$lib/components/Stepper.svelte";
  import { formatMinutes } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/index.svelte";
  import { fade } from "$lib/motion";
  import { succeeded } from "$lib/state/api.svelte";
  import { clock } from "$lib/state/clock.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";
  import { COOLDOWN, limitPhase, STICKS, WINDOW } from "./limit";
  import LimitStatus from "./LimitStatus.svelte";

  type Choice = number | "other";

  const t = $derived(i18n.t.limit);
  const limit = $derived(snapshot.current.limit);
  const saved = $derived(limit.settings);
  const cooling = $derived(limitPhase(limit, clock.now) === "cooldown");

  let draft = $state<LimitSettings>({ ...snapshot.current.limit.settings });
  let dirty = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let savedFlash = $state(false);
  let flashTimer: ReturnType<typeof setTimeout> | undefined;
  let toggling = $state(false);
  let customCooldown = $state(false);
  let customWindow = $state(false);

  $effect(() => {
    const next = saved;
    if (!dirty) draft = { ...next };
  });

  // During a break only stricter values are accepted.
  const sticksMax = $derived(cooling ? saved.sticks : STICKS.max);
  const cooldownMin = $derived(cooling ? saved.cooldownMinutes : COOLDOWN.min);
  const windowMin = $derived(cooling ? saved.windowHours : WINDOW.min);

  const isCooldownPreset = $derived(COOLDOWN.presets.some((m) => m === draft.cooldownMinutes));
  const cooldownChoice = $derived<Choice>(
    customCooldown || !isCooldownPreset ? "other" : draft.cooldownMinutes,
  );
  const cooldownOptions = $derived([
    ...COOLDOWN.presets.map((m) => ({
      value: m as Choice,
      label: formatMinutes(m),
      disabled: m < cooldownMin,
    })),
    {
      value: "other" as Choice,
      label: isCooldownPreset ? t.other : t.custom(formatMinutes(draft.cooldownMinutes)),
    },
  ]);

  const isWindowPreset = $derived(WINDOW.presets.some((h) => h === draft.windowHours));
  const windowChoice = $derived<Choice>(
    customWindow || !isWindowPreset ? "other" : draft.windowHours,
  );
  const windowOptions = $derived([
    ...WINDOW.presets.map((h) => ({
      value: h as Choice,
      label: t.hours(h),
      disabled: h < windowMin,
    })),
    {
      value: "other" as Choice,
      label: isWindowPreset ? t.other : t.custom(t.hours(draft.windowHours)),
    },
  ]);

  function edit(patch: Partial<LimitSettings>, delay = 350): void {
    draft = { ...draft, ...patch };
    dirty = true;
    clearTimeout(timer);
    timer = setTimeout(commit, delay);
  }

  async function commit(): Promise<boolean> {
    const settings = { ...draft };
    const ok = await succeeded((api) => api.setLimitSettings({ settings }));
    dirty = false;
    if (ok) {
      savedFlash = true;
      clearTimeout(flashTimer);
      flashTimer = setTimeout(() => (savedFlash = false), 1600);
    } else {
      draft = { ...saved };
    }
    return ok;
  }

  async function toggle(enabled: boolean): Promise<void> {
    toggling = true;
    clearTimeout(timer);
    draft = { ...draft, enabled };
    dirty = true;
    await commit();
    toggling = false;
  }

  function setCooldown(minutes: number): void {
    edit({ cooldownMinutes: Math.min(COOLDOWN.max, Math.max(cooldownMin, minutes)) });
  }

  function pickCooldown(choice: Choice): void {
    customCooldown = choice === "other";
    if (choice !== "other") edit({ cooldownMinutes: choice }, 0);
  }

  function pickWindow(choice: Choice): void {
    customWindow = choice === "other";
    if (choice !== "other") edit({ windowHours: choice }, 0);
  }
</script>

<Page title={t.title}>
  <LimitStatus onenable={() => toggle(true)} enabling={toggling && !saved.enabled} />

  <section class="settings" aria-labelledby="limit-settings">
    <header>
      <h2 id="limit-settings">{t.settings}</h2>
      {#if savedFlash}
        <span class="saved" role="status" transition:fade={{ duration: 180 }}>
          <Icon name="check" size={13} />{t.saved}
        </span>
      {/if}
    </header>

    <div class="card">
      <div class="sentence">
        <span>{t.after}</span>
        <Stepper
          value={draft.sticks}
          min={STICKS.min}
          max={sticksMax}
          label={t.sticksLabel}
          decreaseLabel={t.decrease}
          increaseLabel={t.increase}
          onchange={(sticks) => edit({ sticks })}
        />
        <span>{t.sticksWord(draft.sticks)} {t.lockFor}</span>
      </div>

      <ChoiceChips
        label={t.cooldownLabel}
        options={cooldownOptions}
        value={cooldownChoice}
        onchange={pickCooldown}
      />

      {#if cooldownChoice === "other"}
        <div class="custom">
          <Stepper
            value={Math.floor(draft.cooldownMinutes / 60)}
            min={Math.floor(cooldownMin / 60)}
            max={24}
            label={t.hoursLabel}
            decreaseLabel={t.decrease}
            increaseLabel={t.increase}
            onchange={(h) => setCooldown(h * 60 + (h === 24 ? 0 : draft.cooldownMinutes % 60))}
          />
          <span>{t.hoursUnit}</span>
          <Stepper
            value={draft.cooldownMinutes % 60}
            min={0}
            max={55}
            step={5}
            label={t.minutesLabel}
            decreaseLabel={t.decrease}
            increaseLabel={t.increase}
            onchange={(m) => setCooldown(Math.floor(draft.cooldownMinutes / 60) * 60 + m)}
          />
          <span>{t.minutesUnit}</span>
        </div>
      {/if}

      {#if cooling}
        <p class="reason"><Icon name="lock" size={13} />{t.strictOnly}</p>
      {/if}

      <div class="more">
        <Disclosure title={t.advanced}>
          <div class="sentence">
            <span>{t.windowLead}</span>
          </div>
          <div class="window">
            <ChoiceChips
              label={t.windowLabel}
              options={windowOptions}
              value={windowChoice}
              onchange={pickWindow}
            />
            {#if windowChoice === "other"}
              <span class="custom">
                <Stepper
                  value={draft.windowHours}
                  min={windowMin}
                  max={WINDOW.max}
                  width={3}
                  label={t.windowLabel}
                  decreaseLabel={t.decrease}
                  increaseLabel={t.increase}
                  onchange={(windowHours) => edit({ windowHours })}
                />
                <span>{t.hoursUnit}</span>
              </span>
            {/if}
          </div>
          <p class="tail">{t.windowTail}</p>
        </Disclosure>
      </div>
    </div>

    {#if saved.enabled}
      <div class="off">
        <Button
          size="small"
          variant="plain"
          disabled={cooling}
          loading={toggling}
          onclick={() => toggle(false)}
        >
          {t.turnOff}
        </Button>
        {#if cooling}<span>{t.offDuringBreak}</span>{/if}
      </div>
    {/if}
  </section>

  <div class="how">
    <Disclosure title={t.how}>
      <p>{t.howBody}</p>
    </Disclosure>
  </div>
</Page>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 24px;
    padding: 0 4px;
  }
  h2 {
    font: var(--text-headline);
    color: var(--label);
  }
  .saved {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font: var(--text-callout);
    color: var(--green);
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px 24px;
    border-radius: var(--radius-group);
    background: var(--group);
    box-shadow: var(--surface-edge);
  }
  .sentence {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 10px;
    font: 400 16px/1.3 var(--font);
    color: var(--label);
  }
  .custom {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    color: var(--label-2);
  }
  .reason {
    display: flex;
    align-items: flex-start;
    gap: 7px;
    max-width: 62ch;
    font: var(--text-callout);
    color: var(--label-2);
  }
  .reason :global(.icon) {
    flex: none;
    margin-top: 1.5px;
    color: var(--label-3);
  }
  .more {
    padding-top: 12px;
    border-top: 0.5px solid var(--hairline);
  }
  .more .sentence {
    font-size: 14px;
    margin-bottom: 8px;
  }
  .window {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 14px;
  }
  .tail {
    margin-top: 8px;
    font: 400 14px/1.3 var(--font);
    color: var(--label);
  }
  .off {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
    padding: 2px 0 0 0;
    font: var(--text-callout);
    color: var(--label-3);
  }
  .how {
    padding: 0 4px;
  }
  .how p {
    max-width: 64ch;
    font: var(--text-callout);
    color: var(--label-2);
  }
</style>
