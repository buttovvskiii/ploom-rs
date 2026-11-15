/** Optional custom model: `public/models/ploom.glb`, if the app was built with one. */
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import type { Finish } from "./finishes";
import { DEVICE, type DeviceModel } from "./model";
import { softSpot } from "./textures";

export const CUSTOM_MODEL_URL = `${import.meta.env.BASE_URL}models/ploom.glb`;

/** "glTF" as a little-endian uint32: the first four bytes of every GLB. */
const GLB_MAGIC = 0x46546c67;

/** Fetch and parse the custom model. */
export async function loadCustomModel(
  url: string,
  signal: AbortSignal,
): Promise<DeviceModel | null> {
  let buffer: ArrayBuffer;
  try {
    const response = await fetch(url, { signal, cache: "no-cache" });
    if (!response.ok || /text\/html/i.test(response.headers.get("content-type") ?? "")) return null;
    buffer = await response.arrayBuffer();
  } catch {
    return null;
  }
  if (buffer.byteLength < 12 || new DataView(buffer).getUint32(0, true) !== GLB_MAGIC) return null;
  if (signal.aborted) return null;

  try {
    const base = new URL(".", new URL(url, location.href)).href;
    const gltf = await new GLTFLoader().parseAsync(buffer, base);
    return fromScene(gltf.scene);
  } catch {
    // A broken file falls back to the built-in model.
    return null;
  }
}

function fromScene(scene: THREE.Object3D): DeviceModel {
  // Stand the longest axis up.
  const box = new THREE.Box3().setFromObject(scene);
  const size = box.getSize(new THREE.Vector3());
  const upright = new THREE.Group();
  upright.add(scene);
  if (size.x > size.y && size.x >= size.z) upright.rotation.z = Math.PI / 2;
  else if (size.z > size.y && size.z > size.x) upright.rotation.x = -Math.PI / 2;
  upright.updateMatrixWorld(true);

  // Scale to the real height, centre on the vertical axis, stand on y = 0.
  const fitted = new THREE.Box3().setFromObject(upright);
  const fittedSize = fitted.getSize(new THREE.Vector3());
  const scale = fittedSize.y > 0 ? DEVICE.height / fittedSize.y : 1;
  const center = fitted.getCenter(new THREE.Vector3());
  const root = new THREE.Group();
  root.name = "CustomDevice";
  upright.scale.setScalar(scale);
  upright.position.set(-center.x * scale, -fitted.min.y * scale, -center.z * scale);
  root.add(upright);
  root.updateMatrixWorld(true);

  // Status light: drive the emissive of whatever the node renders with.
  const lampNode = root.getObjectByName("StatusLight");
  const lampMaterials: THREE.MeshStandardMaterial[] = [];
  const replaced: THREE.Material[] = [];
  lampNode?.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    const own = (Array.isArray(object.material) ? object.material : [object.material]).map((m) => {
      replaced.push(m);
      const lamp =
        m instanceof THREE.MeshStandardMaterial
          ? m.clone()
          : new THREE.MeshStandardMaterial({ color: 0x050505, roughness: 0.2 });
      lamp.emissive.setRGB(0, 0, 0);
      lampMaterials.push(lamp);
      return lamp;
    });
    object.material = Array.isArray(object.material) ? own : (own[0] ?? object.material);
  });

  let halo: THREE.Sprite | null = null;
  const spot = lampNode ? softSpot() : null;
  if (lampNode && spot) {
    halo = new THREE.Sprite(
      new THREE.SpriteMaterial({
        map: spot,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
        transparent: true,
        opacity: 0,
      }),
    );
    const lampBox = new THREE.Box3().setFromObject(lampNode);
    halo.position.copy(lampBox.getCenter(new THREE.Vector3()));
    halo.scale.setScalar(8);
    root.add(halo);
  }

  return {
    root,
    procedural: false,
    setFinish(_finish: Finish): void {
      // A custom model keeps its own materials.
    },
    setStatus(color: THREE.Color, intensity: number): void {
      for (const m of lampMaterials) {
        m.emissive.copy(color);
        m.emissiveIntensity = 2.4 * intensity;
      }
      if (halo) {
        const material = halo.material;
        material.color.copy(color);
        material.opacity = 0.45 * intensity;
      }
    },
    setGlow(): void {
      // Nothing is known about the model's opening.
    },
    setDim(): void {
      // Dimming is done with exposure by the scene.
    },
    update(): boolean {
      return false;
    },
    dispose(): void {
      root.traverse((object) => {
        if (object instanceof THREE.Mesh) {
          object.geometry.dispose();
          const materials = Array.isArray(object.material) ? object.material : [object.material];
          for (const m of materials) {
            for (const value of Object.values(m))
              if (value instanceof THREE.Texture) value.dispose();
            m.dispose();
          }
        }
      });
      for (const m of replaced) m.dispose();
      halo?.material.dispose();
      spot?.dispose();
    },
  };
}
