// `pnpm tauri <command>`: the Tauri CLI, pointed at ../src-tauri.
import { fileURLToPath } from "node:url";
import { run } from "@tauri-apps/cli";

process.env.TAURI_APP_PATH ??= fileURLToPath(new URL("../../src-tauri", import.meta.url));

run(process.argv.slice(2), "pnpm tauri").catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
