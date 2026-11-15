/**
 * Hand-drawn glyphs on a 20×20 grid, stroked at 1.5 with round joins, in the spirit of SF Symbols.
 */

const rr = (x: number, y: number, w: number, h: number, r: number): string =>
  `M${x + r} ${y}h${w - 2 * r}a${r} ${r} 0 0 1 ${r} ${r}v${h - 2 * r}a${r} ${r} 0 0 1 -${r} ${r}` +
  `h-${w - 2 * r}a${r} ${r} 0 0 1 -${r} -${r}v-${h - 2 * r}a${r} ${r} 0 0 1 ${r} -${r}z`;

const circle = (cx: number, cy: number, r: number): string =>
  `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 -${2 * r} 0z`;

export interface Glyph {
  stroke: string[];
  fill?: string[];
}

export const GLYPHS = {
  overview: {
    stroke: [
      "M3.2 14.2a7 7 0 1 1 13.6 0",
      "M10 12.4l3.1-4.1",
      "M5.4 9.4l-.9-.5M10 5.6V4.6M14.6 9.4l.9-.5",
    ],
    fill: [circle(10, 12.6, 1.35)],
  },
  heating: {
    stroke: [
      "M10 17.6c-2.9 0-4.9-2-4.9-4.8 0-2.5 1.5-3.9 2.6-5.6.3 1.2.9 2.1 1.8 2.6.1-2.9 1.4-5 3.4-7.2.4 2.9 2.9 4.7 2.9 9 0 3.6-2.4 6-5.8 6z",
      "M10 17.6c-1.2 0-2-.9-2-2.1 0-1.2.8-1.9 1.4-2.8.5.9 1 1.2 1.6 1.5.6.4 1 1 1 1.6 0 1-.8 1.8-2 1.8z",
    ],
  },
  history: {
    stroke: ["M3.6 10a6.4 6.4 0 1 0 1.9-4.5L3.4 7.6", "M3.4 4.4v3.2h3.2", "M10 6.6V10l2.5 1.7"],
  },
  diagnostics: {
    stroke: ["M2.4 10.4h3.1l1.7-4.1 2.7 8.6 2-6.2 1.4 1.7h4.3"],
  },
  devices: {
    stroke: [
      "M7.1 7.1a4.1 4.1 0 0 0 0 5.8",
      "M12.9 7.1a4.1 4.1 0 0 1 0 5.8",
      "M4.7 4.7a7.5 7.5 0 0 0 0 10.6",
      "M15.3 4.7a7.5 7.5 0 0 1 0 10.6",
    ],
    fill: [circle(10, 10, 1.35)],
  },
  console: {
    stroke: [rr(2.5, 3.5, 15, 13, 2.6), "M6.2 8.2 8.6 10l-2.4 1.8", "M10.4 12.2h3.6"],
  },
  lock: {
    stroke: [rr(5, 9, 10, 8.2, 2), "M7.2 9V6.6a2.8 2.8 0 0 1 5.6 0V9"],
  },
  limit: {
    stroke: [
      "M5.6 3h8.8M5.6 17h8.8",
      "M6.6 3v2.2c0 1.9 3.4 3.1 3.4 4.8s-3.4 2.9-3.4 4.8V17",
      "M13.4 3v2.2c0 1.9-3.4 3.1-3.4 4.8s3.4 2.9 3.4 4.8V17",
    ],
  },
  bolt: {
    stroke: [],
    fill: ["M11.4 2.2 4.8 11h4.4l-.9 6.8L15 9h-4.5z"],
  },
  thermometer: {
    stroke: ["M10 3a1.8 1.8 0 0 0-1.8 1.8v7.2a3.3 3.3 0 1 0 3.6 0V4.8A1.8 1.8 0 0 0 10 3z"],
    fill: ["M9.3 9h1.4v4.2H9.3z", circle(10, 14.6, 1.6)],
  },
  vibrate: {
    stroke: [rr(6.6, 3.2, 6.8, 13.6, 1.8), "M3.6 7.2v5.6M16.4 7.2v5.6"],
  },
  check: {
    stroke: ["M4.8 10.6l3.2 3.2 7.2-7.6"],
  },
  close: {
    stroke: ["M6 6l8 8M14 6l-8 8"],
  },
  warning: {
    stroke: [
      "M10 3.4c.4 0 .7.2.9.5l6.3 11c.4.7-.1 1.6-.9 1.6H3.7c-.8 0-1.3-.9-.9-1.6l6.3-11c.2-.3.5-.5.9-.5z",
      "M10 8v3.4",
    ],
    fill: [circle(10, 13.7, 0.95)],
  },
  info: {
    stroke: [circle(10, 10, 7), "M10 9.2v4.6"],
    fill: [circle(10, 6.6, 0.95)],
  },
  device: {
    stroke: [rr(6.4, 2.2, 7.2, 15.6, 3.6)],
    fill: [circle(10, 6.4, 1.1)],
  },
  refresh: {
    stroke: ["M15.6 10a5.6 5.6 0 1 1-1.7-4", "M14.4 2.8v3.4H11"],
  },
  globe: {
    stroke: [
      circle(10, 10, 7),
      "M3 10h14",
      "M10 3c-2 2-2.8 4.3-2.8 7s.8 5 2.8 7c2-2 2.8-4.3 2.8-7S12 5 10 3z",
    ],
  },
  plug: {
    stroke: ["M7.4 3v3.6M12.6 3v3.6", "M5.4 6.6h9.2V9a4.6 4.6 0 0 1-9.2 0z", "M10 13.6V17"],
  },
  chevronRight: {
    stroke: ["M8 5.5 12.5 10 8 14.5"],
  },
  frames: {
    stroke: ["M4 5h12M4 8.3h12M4 11.7h12M4 15h8"],
  },
} satisfies Record<string, Glyph>;

export type IconName = keyof typeof GLYPHS;
