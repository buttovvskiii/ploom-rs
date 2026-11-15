<script lang="ts">
  import { flip } from "svelte/animate";
  import type { ActivityEvent } from "$lib/api";
  import Icon from "$lib/components/icons/Icon.svelte";
  import { formatRelative, formatTime } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/index.svelte";
  import { easeOut, reducedMotion, shift } from "$lib/motion";
  import { clock } from "$lib/state/clock.svelte";
  import { describe, iconFor, isAlert } from "./describe";

  interface Props {
    events: ActivityEvent[];
  }
  let { events }: Props = $props();
  const t = $derived(i18n.t);
  // New events slide in at the top and push the older ones down.
  const move = $derived({ duration: reducedMotion() ? 0 : 320, easing: easeOut });
</script>

{#if events.length === 0}
  <p class="empty">{t.feed.empty}</p>
{:else}
  <ol class="feed">
    {#each events as event (event.id)}
      <li class:alert={isAlert(event)} animate:flip={move} in:shift={{ y: -8, duration: 320 }}>
        <Icon name={iconFor(event)} size={14} />
        <span class="text">{describe(event)}</span>
        <time
          class="num"
          datetime={new Date(event.timestampMs).toISOString()}
          title={formatTime(event.timestampMs)}
        >
          {formatRelative(event.timestampMs, clock.now)}
        </time>
      </li>
    {/each}
  </ol>
{/if}

<style>
  .feed {
    list-style: none;
    margin: 0;
    padding: 4px 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 32px;
    padding: 0 var(--row-x);
  }
  li :global(.icon) {
    color: var(--label-3);
  }
  .alert :global(.icon) {
    color: var(--orange);
  }
  .text {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  time {
    flex: none;
    font: var(--text-callout);
    color: var(--label-3);
  }
  .empty {
    padding: 14px var(--row-x);
    color: var(--label-2);
    font: var(--text-callout);
  }
</style>
