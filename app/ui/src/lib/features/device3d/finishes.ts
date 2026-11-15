/** The body colors of the Ploom Aura (GEN4L) catalog. */
import type { DeviceIdentity } from "$lib/api";

export type CatalogFinish = "black" | "silver" | "gold" | "blue" | "glacier" | "teal";
/** `neutral` stands in for a device whose color is not known. */
export type FinishId = CatalogFinish | "neutral";

const CATALOG: readonly CatalogFinish[] = ["black", "silver", "gold", "blue", "glacier", "teal"];

export const DEFAULT_FINISH: FinishId = "neutral";

export function isCatalogFinish(value: unknown): value is CatalogFinish {
  return CATALOG.some((f) => f === value);
}

export function isFinish(value: unknown): value is FinishId {
  return value === "neutral" || isCatalogFinish(value);
}

/** The catalog color the device reports through its hardware string, if any. */
export function detectedFinish(device: DeviceIdentity | null | undefined): CatalogFinish | null {
  const id = device?.color?.id;
  return isCatalogFinish(id) ? id : null;
}

/** A physically based surface, in the subset of MeshPhysicalMaterial we use. */
export interface Surface {
  /** sRGB hex. */
  color: string;
  metalness: number;
  roughness: number;
  clearcoat: number;
  clearcoatRoughness: number;
  /** Specular strength for dielectrics (1 = the default 4% reflectance). */
  specularIntensity: number;
}

export interface Finish {
  /** Catalog base color (sRGB). */
  base: string;
  /** Satin body. */
  body: Surface;
  /** Reflective front cover. */
  cover: Surface;
  /** The swatch and the static placeholder: [light, dark] sRGB. */
  swatch: readonly [string, string];
  /** Placeholder cover tone. */
  swatchCover: string;
  /** Exposure for this finish, so light colors keep their form instead of washing out. */
  exposure: number;
}

const satin = (color: string, metalness: number, roughness = 0.42): Surface => ({
  color,
  metalness,
  roughness,
  clearcoat: 0,
  clearcoatRoughness: 0,
  specularIntensity: metalness > 0.5 ? 1 : 0.7,
});

const gloss = (color: string, metalness: number, roughness = 0.14): Surface => ({
  color,
  metalness,
  roughness,
  clearcoat: metalness > 0.8 ? 0.5 : 1,
  clearcoatRoughness: 0.04,
  specularIntensity: 1,
});

export const FINISH_SPECS: Record<FinishId, Finish> = {
  // Graphite: a satin near-black body under a deep, glossy cover.
  black: {
    base: "#0b0b0b",
    body: satin("#242427", 0.6, 0.46),
    cover: gloss("#09090a", 0.45, 0.16),
    swatch: ["#3a3a3c", "#0b0b0b"],
    swatchCover: "#19191a",
    exposure: 1.08,
  },
  // Metallic: the base is lifted a little so brushed metal reads as silver, not grey.
  silver: {
    base: "#848385",
    body: satin("#9b9a9d", 0.9, 0.36),
    cover: gloss("#b4b3b6", 1, 0.1),
    swatch: ["#d9d8db", "#7a797c"],
    swatchCover: "#bdbcbf",
    exposure: 0.92,
  },
  // Warm rose/champagne metal.
  gold: {
    base: "#af857e",
    body: satin("#b58b83", 0.88, 0.37),
    cover: gloss("#cfa69d", 1, 0.1),
    swatch: ["#e5c3bb", "#9c726b"],
    swatchCover: "#d4aca3",
    exposure: 0.95,
  },
  // Deep navy paint.
  blue: {
    base: "#061832",
    body: satin("#0b1f3d", 0.3),
    cover: gloss("#061832", 0.5, 0.15),
    swatch: ["#2d4a78", "#061832"],
    swatchCover: "#132a50",
    exposure: 1.08,
  },
  // Cool pearl white: soft satin paint, a lacquered cover with a faint cool sheen.
  glacier: {
    base: "#dce0df",
    body: { ...satin("#d0d6d5", 0, 0.38), specularIntensity: 1 },
    cover: gloss("#e1e7e7", 0.05, 0.09),
    swatch: ["#f4f6f6", "#bfc5c4"],
    swatchCover: "#e9eded",
    exposure: 0.84,
  },
  // Limited edition: muted teal.
  teal: {
    base: "#699e85",
    body: satin("#699e85", 0.3, 0.4),
    cover: gloss("#5f957c", 0.45, 0.14),
    swatch: ["#9fcab5", "#4f7f69"],
    swatchCover: "#73a990",
    exposure: 1,
  },
  neutral: {
    base: "#8a8a8d",
    body: satin("#6f6f73", 0.55, 0.44),
    cover: gloss("#85858a", 0.6, 0.14),
    swatch: ["#a9a9ad", "#5c5c60"],
    swatchCover: "#8a8a8e",
    exposure: 1,
  },
};
