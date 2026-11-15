import type { Catalog, HeatingMode, ModeInfo } from "$lib/api";

class CatalogStore {
  current = $state<Catalog | null>(null);

  mode(id: HeatingMode | null | undefined): ModeInfo | null {
    if (!id) return null;
    return this.current?.modes.find((m) => m.id === id) ?? null;
  }

  dangerFor(opcode: number) {
    return this.current?.dangerousOpcodes.find((d) => d.opcode === opcode) ?? null;
  }
}

export const catalog = new CatalogStore();
