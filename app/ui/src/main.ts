import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { isTauri } from "$lib/api";
import { prewarmViewer } from "$lib/features/device3d/host";
import { prefs } from "$lib/state/prefs.svelte";

// In the desktop shell on macOS the window is transparent with native sidebar vibrancy; let it show
// through the sidebar.
if (isTauri && /Mac/.test(navigator.platform || navigator.userAgent)) {
  document.documentElement.dataset.vibrancy = "";
}

window.addEventListener("contextmenu", (event) => {
  const el = event.target instanceof Element ? event.target : null;
  if (!el?.closest("input, textarea, [contenteditable]")) event.preventDefault();
});

const target = document.getElementById("app");
if (!target) throw new Error("#app missing");

const app = mount(App, { target });

// Load the 3D scene while the first screen settles, so the device is ready when it is shown.
if (isTauri) setTimeout(() => prewarmViewer(prefs.finishFor(null)), 0);

export default app;
