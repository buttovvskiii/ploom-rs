/** The UI-facing API. */
import type { ActivityEvent, CommandError, Commands, ErrorCode, Snapshot } from "./types";

export type CommandName = keyof Commands;
export type CommandArgs<K extends CommandName> = Commands[K]["args"];
export type CommandResult<K extends CommandName> = Commands[K]["result"];

/** `read_diagnostics` → `readDiagnostics` */
type CamelCase<S extends string> = S extends `${infer Head}_${infer Tail}`
  ? `${Head}${Capitalize<CamelCase<Tail>>}`
  : S;

type NoArgs = Record<string, never>;

export type CommandMethods = {
  [K in CommandName as CamelCase<K>]: CommandArgs<K> extends NoArgs
    ? () => Promise<CommandResult<K>>
    : (args: CommandArgs<K>) => Promise<CommandResult<K>>;
};

export type Unsubscribe = () => void;

export interface ApiClient extends CommandMethods {
  onSnapshot(handler: (snapshot: Snapshot) => void): Unsubscribe;
  onActivity(handler: (event: ActivityEvent) => void): Unsubscribe;
}

/** What a backend has to provide; `createClient` builds the typed surface on top. */
export interface Backend {
  invoke<K extends CommandName>(name: K, args: CommandArgs<K>): Promise<CommandResult<K>>;
  onSnapshot(handler: (snapshot: Snapshot) => void): Unsubscribe;
  onActivity(handler: (event: ActivityEvent) => void): Unsubscribe;
}

const ERROR_CODES: Record<ErrorCode, true> = {
  notConnected: true,
  busy: true,
  deviceNotFound: true,
  bluetoothUnavailable: true,
  timeout: true,
  deviceRejected: true,
  calibrationOutOfRange: true,
  invalidInput: true,
  dangerousFrame: true,
  cooldownActive: true,
  internal: true,
};

export function isErrorCode(value: unknown): value is ErrorCode {
  return typeof value === "string" && Object.hasOwn(ERROR_CODES, value);
}

/** Every rejected command surfaces as this, whatever the backend threw. */
export class ApiError extends Error implements CommandError {
  readonly code: ErrorCode;
  readonly detail: string;

  constructor(code: ErrorCode, detail: string) {
    super(`${code}: ${detail}`);
    this.name = "ApiError";
    this.code = code;
    this.detail = detail;
  }

  static from(error: unknown): ApiError {
    if (error instanceof ApiError) return error;
    if (typeof error === "object" && error !== null && "code" in error) {
      const { code, detail } = error as { code: unknown; detail?: unknown };
      if (isErrorCode(code)) return new ApiError(code, typeof detail === "string" ? detail : "");
    }
    if (typeof error === "string") return new ApiError("internal", error);
    if (error instanceof Error) return new ApiError("internal", error.message);
    return new ApiError("internal", String(error));
  }
}

/** Exhaustive by construction: adding a command to `Commands` fails to compile here. */
const COMMAND_NAMES: Record<CommandName, true> = {
  get_snapshot: true,
  get_catalog: true,
  get_activity: true,
  scan: true,
  connect: true,
  disconnect: true,
  set_vibration: true,
  set_lock: true,
  set_auto_start: true,
  set_limit_settings: true,
  apply_profile: true,
  preview_profile: true,
  read_logs: true,
  read_diagnostics: true,
  send_raw: true,
};

function camel(name: string): string {
  return name.replace(/_([a-z])/g, (_, c: string) => c.toUpperCase());
}

export function createClient(backend: Backend): ApiClient {
  const methods: Record<string, (args?: unknown) => Promise<unknown>> = {};
  for (const name of Object.keys(COMMAND_NAMES) as CommandName[]) {
    methods[camel(name)] = async (args?: unknown) => {
      try {
        return await backend.invoke(name, (args ?? {}) as CommandArgs<typeof name>);
      } catch (error) {
        throw ApiError.from(error);
      }
    };
  }
  return {
    ...(methods as unknown as CommandMethods),
    onSnapshot: (handler) => backend.onSnapshot(handler),
    onActivity: (handler) => backend.onActivity(handler),
  };
}
