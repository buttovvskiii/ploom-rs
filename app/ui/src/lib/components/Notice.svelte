<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./icons/Icon.svelte";
  import type { IconName } from "./icons/paths";

  interface Props {
    icon?: IconName;
    tone?: "neutral" | "warning";
    children: Snippet;
  }
  let { icon = "info", tone = "neutral", children }: Props = $props();
</script>

<div class="notice {tone}">
  <Icon name={icon} size={15} />
  <div class="text">{@render children()}</div>
</div>

<style>
  .notice {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    padding: 0 4px;
    font: var(--text-callout);
    color: var(--label-2);
  }
  .notice :global(.icon) {
    margin-top: 0.5px;
    color: var(--label-3);
  }
  .warning :global(.icon) {
    color: var(--orange);
  }
  .text {
    max-width: 64ch;
  }
</style>
