# Custom 3D model

Drop a file named `ploom.glb` in this folder and the Overview page will load it
instead of the built-in procedural device. Nothing else is required — the app
checks for the file at startup and falls back to the built-in model if it is
missing or fails to load.

## Format

- **glTF binary (`.glb`)**, a single self-contained file.
- **Y-up**, any size or centring — the app auto-centres the model and scales it
  to the device's real height (109.4 mm). Orientation is normalised so the
  longest axis stands upright.
- **PBR materials** (metalness/roughness). They are lit by the same studio
  environment as the built-in model, so author them for physically based
  rendering. The custom model keeps its own materials, so the colour detected
  from the device is not applied to it.
- Ideally **under 5 MB** and reasonably low-poly — this renders in a desktop
  WebView, not a game engine.

## Status light (optional)

If the model contains a node named **`StatusLight`**, its material's emissive
is driven by the device state (mode tint while heating, green while charging,
and so on). Without such a node the status light is simply skipped; everything
else still works.

## Where to get one

You can photograph your own device and run it through a photo-to-3D generator
(several online tools and local models do this), then export a `.glb` and drop
it here. No third-party model files are shipped with this project.
