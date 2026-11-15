<script lang="ts">
  import type { HeatingMode, ProfileTarget } from "$lib/api";
  import Group from "$lib/components/Group.svelte";
  import Notice from "$lib/components/Notice.svelte";
  import Page from "$lib/components/Page.svelte";
  import { domainFor } from "$lib/curve/curve";
  import DisconnectedState from "$lib/features/shell/DisconnectedState.svelte";
  import { i18n } from "$lib/i18n/index.svelte";
  import { modeName } from "$lib/modes";
  import { succeeded } from "$lib/state/api.svelte";
  import { catalog } from "$lib/state/catalog.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import ApplySheet from "./ApplySheet.svelte";
  import CustomEditor from "./CustomEditor.svelte";
  import FramesSheet from "./FramesSheet.svelte";
  import ModeRow from "./ModeRow.svelte";

  const t = $derived(i18n.t);
  const status = $derived(snapshot.status);
  const modes = $derived(catalog.current?.modes ?? []);
  const domain = $derived(domainFor(modes.flatMap((m) => (m.curve ? [m.curve] : []))));
  const blocked = $derived(!snapshot.connected || snapshot.busy || snapshot.inSession);

  let applying = $state<ProfileTarget | null>(null);
  let previewing = $state<ProfileTarget | null>(null);
  let switchingBalanced = $state(false);

  async function apply(mode: HeatingMode): Promise<void> {
    if (mode !== "balanced") {
      applying = { kind: "preset", mode };
      return;
    }
    // Balanced is the factory profile: a single frame, no confirmation needed.
    switchingBalanced = true;
    const ok = await succeeded((api) => api.applyProfile({ target: { kind: "preset", mode } }));
    switchingBalanced = false;
    if (ok) toasts.show(t.heating.sheet.successBody(modeName("balanced")));
  }
</script>

<Page title={t.heating.title}>
  {#if !snapshot.connected}
    <DisconnectedState compact />
  {:else if snapshot.inSession}
    <Notice icon="heating">{t.heating.sheet.busyHeating}</Notice>
  {/if}

  <Group title={t.heating.modes} footer={t.heating.modesFooter}>
    {#each modes as mode (mode.id)}
      <ModeRow
        {mode}
        {domain}
        active={status.heatingMode === mode.id}
        activeCustom={status.heatingMode === mode.id && status.customProfile !== null}
        disabled={blocked}
        loading={mode.id === "balanced" && switchingBalanced}
        onapply={() => apply(mode.id)}
      />
    {/each}
  </Group>

  <CustomEditor
    disabled={blocked}
    onapply={(target) => (applying = target)}
    onpreview={(target) => (previewing = target)}
  />
</Page>

<ApplySheet target={applying} onclose={() => (applying = null)} />
<FramesSheet target={previewing} onclose={() => (previewing = null)} />
