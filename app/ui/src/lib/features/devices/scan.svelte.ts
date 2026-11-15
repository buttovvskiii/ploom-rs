import type { DiscoveredDevice } from "$lib/api";
import { run } from "$lib/state/api.svelte";

class ScanStore {
  results = $state<DiscoveredDevice[] | null>(null);
  pending = $state(false);

  async scan(): Promise<void> {
    if (this.pending) return;
    this.pending = true;
    const found = await run((api) => api.scan());
    if (found) this.results = [...found].sort((a, b) => (b.rssiDbm ?? -999) - (a.rssiDbm ?? -999));
    this.pending = false;
  }
}

export const scanner = new ScanStore();
