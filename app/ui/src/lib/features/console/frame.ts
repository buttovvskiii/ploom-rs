/** Live validation of a raw `[LEN][OPCODE][payload]` frame typed as hex. */
export type ParsedFrame =
  | { ok: true; hex: string; opcode: number; payloadLength: number }
  | { ok: false; reason: "empty" | "chars" | "odd" | "short" }
  | { ok: false; reason: "len"; expected: number; actual: number };

export function parseFrame(input: string): ParsedFrame {
  const hex = input.replace(/\s+/g, "").replace(/^0x/i, "").toLowerCase();
  if (hex.length === 0) return { ok: false, reason: "empty" };
  if (!/^[0-9a-f]*$/.test(hex)) return { ok: false, reason: "chars" };
  if (hex.length % 2 !== 0) return { ok: false, reason: "odd" };
  const bytes = hex.match(/../g)?.map((b) => parseInt(b, 16)) ?? [];
  const [len, opcode] = bytes;
  if (len === undefined || opcode === undefined) return { ok: false, reason: "short" };
  if (len !== bytes.length - 1)
    return { ok: false, reason: "len", expected: bytes.length - 1, actual: len };
  return { ok: true, hex, opcode, payloadLength: bytes.length - 2 };
}

export const byteHex = (n: number): string => n.toString(16).padStart(2, "0");
