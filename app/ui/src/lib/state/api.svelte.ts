import { ApiError, loadClient, type ApiClient } from "$lib/api";
import { i18n } from "$lib/i18n/index.svelte";
import { activity } from "./activity.svelte";
import { catalog } from "./catalog.svelte";
import { snapshot } from "./snapshot.svelte";
import { toasts } from "./toasts.svelte";

let client = $state<ApiClient | null>(null);
let failure = $state<string | null>(null);

export const backend = {
  get ready(): boolean {
    return client !== null;
  },
  get failure(): string | null {
    return failure;
  },
};

/** The API client. */
export function api(): ApiClient {
  if (!client) throw new Error("API used before start()");
  return client;
}

/** Load the backend, subscribe first (so nothing is missed), then fetch the initial state. */
export async function start(): Promise<void> {
  try {
    const c = loadClient();
    c.onSnapshot((s) => snapshot.set(s));
    c.onActivity((e) => activity.push(e));
    const [snap, cat, history] = await Promise.all([
      c.getSnapshot(),
      c.getCatalog(),
      c.getActivity(),
    ]);
    snapshot.set(snap);
    catalog.current = cat;
    activity.replace(history);
    client = c;
  } catch (error) {
    failure = ApiError.from(error).detail || String(error);
  }
}

/** Run a command and turn a rejection into a localized toast. */
export async function run<T>(
  command: (c: ApiClient) => Promise<T>,
  options: { rethrow?: boolean } = {},
): Promise<T | undefined> {
  try {
    return await command(api());
  } catch (error) {
    const e = ApiError.from(error);
    if (options.rethrow) throw e;
    toasts.show(i18n.t.errors[e.code], { detail: e.detail, tone: "error" });
    return undefined;
  }
}

/** Like `run`, for commands without a result: resolves to whether it succeeded. */
export async function succeeded(command: (c: ApiClient) => Promise<unknown>): Promise<boolean> {
  let ok = false;
  await run(async (c) => {
    await command(c);
    ok = true;
  });
  return ok;
}
