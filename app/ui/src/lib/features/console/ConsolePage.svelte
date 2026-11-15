<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Group from "$lib/components/Group.svelte";
  import Page from "$lib/components/Page.svelte";
  import Sheet from "$lib/components/Sheet.svelte";
  import Icon from "$lib/components/icons/Icon.svelte";
  import DisconnectedState from "$lib/features/shell/DisconnectedState.svelte";
  import { formatOpcode, formatTime, spacedHex } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/index.svelte";
  import { run } from "$lib/state/api.svelte";
  import { catalog } from "$lib/state/catalog.svelte";
  import { snapshot } from "$lib/state/snapshot.svelte";
  import { exchanges } from "./exchanges.svelte";
  import { byteHex, parseFrame } from "./frame";

  const t = $derived(i18n.t);

  let input = $state("");
  let sending = $state(false);
  let confirming = $state<{ hex: string; opcode: number } | null>(null);

  const parsed = $derived(parseFrame(input));
  const danger = $derived(parsed.ok ? catalog.dangerFor(parsed.opcode) : null);

  const message = $derived.by((): { text: string; tone: "ok" | "error" | "hint" } => {
    const v = t.console;
    if (parsed.ok)
      return { text: v.valid(formatOpcode(parsed.opcode), parsed.payloadLength), tone: "ok" };
    if (parsed.reason === "empty") return { text: v.invalid.empty, tone: "hint" };
    if (parsed.reason === "len") {
      return {
        text: v.invalid.len(byteHex(parsed.expected), byteHex(parsed.actual)),
        tone: "error",
      };
    }
    return { text: v.invalid[parsed.reason], tone: "error" };
  });

  const presets = $derived([
    { label: t.console.preset.status, hex: "01 e3" },
    { label: t.console.preset.rssi, hex: "01 fd" },
    { label: t.console.preset.profile, hex: "01 a2" },
    { label: t.console.preset.mode, hex: "01 a3" },
    { label: t.console.preset.enclosure, hex: "04 f1 09 ff ff" },
  ]);

  async function send(hex: string, confirmDangerous: boolean): Promise<void> {
    sending = true;
    const result = await run((api) => api.sendRaw({ hex, confirmDangerous }));
    sending = false;
    if (result) exchanges.add(result);
  }

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    if (!parsed.ok || sending) return;
    if (danger) {
      confirming = { hex: parsed.hex, opcode: parsed.opcode };
      return;
    }
    void send(parsed.hex, false);
  }

  function confirmSend(): void {
    if (!confirming) return;
    const { hex } = confirming;
    confirming = null;
    void send(hex, true);
  }

  const confirmDanger = $derived(confirming ? catalog.dangerFor(confirming.opcode) : null);
  const decodedText = (value: unknown) =>
    typeof value === "string" ? value : JSON.stringify(value);
</script>

<Page title={t.console.title}>
  <p class="intro">{t.console.intro}</p>

  {#if !snapshot.connected}
    <DisconnectedState compact />
  {/if}

  <form class="composer" onsubmit={submit}>
    <label class="visually-hidden" for="frame-input">{t.console.frame}</label>
    <div class="field" class:invalid={message.tone === "error"} class:danger={danger !== null}>
      <input
        id="frame-input"
        class="mono"
        bind:value={input}
        placeholder={t.console.placeholder}
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        aria-invalid={message.tone === "error"}
        aria-describedby="frame-status"
      />
      <Button
        type="submit"
        variant={danger ? "destructive" : "primary"}
        size="small"
        loading={sending}
        disabled={!parsed.ok || !snapshot.connected || snapshot.busy}
      >
        {sending ? t.console.sending : t.console.send}
      </Button>
    </div>
    <p id="frame-status" class="status {message.tone}" aria-live="polite">
      {#if danger}
        <Icon name="warning" size={13} />
        <span>{t.console.dangerous}: {t.danger[danger.reason]}</span>
      {:else}
        <span>{message.text}</span>
      {/if}
    </p>
    <div class="presets" role="group" aria-label={t.console.presets}>
      {#each presets as preset (preset.hex)}
        <button type="button" class="chip" title={preset.hex} onclick={() => (input = preset.hex)}>
          {preset.label}
        </button>
      {/each}
    </div>
  </form>

  <Group title={t.console.replies}>
    {#snippet actions()}
      {#if exchanges.items.length}
        <Button variant="plain" size="small" onclick={() => exchanges.clear()}
          >{t.console.clear}</Button
        >
      {/if}
    {/snippet}
    {#if exchanges.items.length === 0}
      <p class="empty">{t.console.emptyLog}</p>
    {:else}
      <ol class="log selectable">
        {#each exchanges.items as ex (ex.id)}
          <li class="exchange">
            <div class="line sent">
              <span class="tag">{t.console.sent}</span>
              <code>{spacedHex(ex.sent)}</code>
              <time class="num">{formatTime(ex.at)}</time>
            </div>
            {#if ex.replies.length === 0}
              <p class="none">{t.console.noReplies}</p>
            {:else}
              {#each ex.replies as reply, i (i)}
                <div class="line">
                  <span class="tag mono">{formatOpcode(reply.opcode)}</span>
                  <code>{spacedHex(reply.hex)}</code>
                </div>
                {#if Object.keys(reply.decoded).length}
                  <dl class="decoded">
                    {#each Object.entries(reply.decoded) as [key, value] (key)}
                      <div>
                        <dt>{key}</dt>
                        <dd>{decodedText(value)}</dd>
                      </div>
                    {/each}
                  </dl>
                {/if}
              {/each}
            {/if}
          </li>
        {/each}
      </ol>
    {/if}
  </Group>
</Page>

<Sheet
  open={confirming !== null}
  title={confirming ? t.console.dangerSheet.title(formatOpcode(confirming.opcode)) : ""}
  onclose={() => (confirming = null)}
>
  {#if confirming && confirmDanger}
    <p class="reason">{t.danger[confirmDanger.reason]}</p>
    <code class="frame">{spacedHex(confirming.hex)}</code>
  {/if}
  {#snippet footer()}
    <Button onclick={() => (confirming = null)} data-autofocus>{t.common.cancel}</Button>
    <Button variant="destructive" onclick={confirmSend}>{t.console.dangerSheet.confirm}</Button>
  {/snippet}
</Sheet>

<style>
  .intro {
    padding: 0 4px;
    font: var(--text-callout);
    color: var(--label-2);
    max-width: 64ch;
  }
  .composer {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 6px 6px 12px;
    border-radius: var(--radius-group);
    background: var(--group);
    box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.06);
    transition: box-shadow 120ms ease;
  }
  .field:focus-within {
    box-shadow:
      inset 0 0 0 0.5px rgba(255, 255, 255, 0.06),
      0 0 0 3px var(--focus);
  }
  .field.invalid {
    box-shadow: inset 0 0 0 1px rgba(255, 159, 10, 0.5);
  }
  .field.danger {
    box-shadow: inset 0 0 0 1px rgba(255, 69, 58, 0.55);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 28px;
    border: 0;
    background: transparent;
    font: 400 14px/1 var(--mono);
    letter-spacing: 0.02em;
    -webkit-user-select: text;
    user-select: text;
  }
  input::placeholder {
    color: var(--label-3);
  }
  input:focus-visible {
    outline: none;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 16px;
    padding: 0 4px;
    font: var(--text-callout);
    color: var(--label-2);
  }
  .status.hint {
    color: var(--label-3);
  }
  .status.error {
    color: var(--orange);
  }
  .status :global(.icon) {
    color: var(--red);
  }
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 0 2px;
  }
  .chip {
    height: 24px;
    padding: 0 10px;
    border: 0;
    border-radius: 12px;
    background: var(--fill);
    color: var(--label-2);
    font: var(--text-callout);
  }
  .chip:hover {
    background: var(--fill-strong);
    color: var(--label);
  }
  .chip:focus-visible {
    border-radius: 12px;
  }
  .empty {
    padding: 14px var(--row-x);
    font: var(--text-callout);
    color: var(--label-2);
  }
  .log {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .exchange {
    position: relative;
    padding: 10px var(--row-x);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .exchange:not(:first-child)::before {
    content: "";
    position: absolute;
    top: 0;
    left: var(--row-x);
    right: 0;
    height: 0.5px;
    background: var(--hairline-strong);
  }
  .line {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }
  .tag {
    flex: none;
    width: 10ch;
    font: var(--text-footnote);
    color: var(--label-3);
  }
  .tag.mono {
    font: 400 11px/1.5 var(--mono);
    color: var(--label-2);
  }
  code,
  .mono {
    font-family: var(--mono);
  }
  .line code {
    flex: 1;
    min-width: 0;
    font: 400 12px/1.5 var(--mono);
    overflow-wrap: anywhere;
  }
  .sent code {
    color: var(--label);
  }
  .line time {
    font: var(--text-footnote);
    color: var(--label-3);
  }
  .none {
    margin-left: calc(10ch + 10px);
    font: var(--text-callout);
    color: var(--label-3);
  }
  .decoded {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 14px;
    margin: 0 0 2px calc(10ch + 10px);
    font: 400 11px/1.5 var(--mono);
  }
  .decoded div {
    display: flex;
    gap: 6px;
  }
  dt {
    color: var(--label-3);
  }
  dd {
    margin: 0;
    color: var(--label-2);
  }
  .reason {
    color: var(--label);
  }
  .frame {
    display: block;
    margin-top: 12px;
    padding: 8px 10px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.22);
    font: 400 12px/1.4 var(--mono);
    color: var(--label);
  }
</style>
