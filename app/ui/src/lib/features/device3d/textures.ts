import * as THREE from "three";

/**
 * Small procedural textures drawn on a 2D canvas: no image files, nothing for the CSP to allow
 * beyond what a canvas needs.
 */

/** A soft radial falloff: white RGB, alpha from `stops` (offset 0 = centre). */
export function radialTexture(stops: readonly (readonly [number, number])[]): THREE.Texture {
  const size = 128;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (ctx) {
    const g = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
    for (const [offset, alpha] of stops) g.addColorStop(offset, `rgba(255,255,255,${alpha})`);
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, size, size);
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.NoColorSpace;
  return texture;
}

/** A Gaussian-like falloff, for shadows and halos. */
export function softSpot(peak = 1): THREE.Texture {
  const stops: [number, number][] = [];
  for (let i = 0; i <= 10; i++) {
    const t = i / 10;
    stops.push([t, peak * Math.exp(-4.2 * t * t) * (1 - t)]);
  }
  return radialTexture(stops);
}

/** Vertical ramp for an emissive map: bright at v = 0 (the bottom of a cylinder). */
export function verticalRamp(): THREE.Texture {
  const canvas = document.createElement("canvas");
  canvas.width = 4;
  canvas.height = 64;
  const ctx = canvas.getContext("2d");
  if (ctx) {
    const g = ctx.createLinearGradient(0, 0, 0, 64);
    g.addColorStop(0, "rgb(0,0,0)");
    g.addColorStop(0.55, "rgb(40,40,40)");
    g.addColorStop(1, "rgb(255,255,255)");
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, 4, 64);
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/** Fine grey noise for a roughness map: the metallic flake of the anodised body. */
export function flakeNoise(): THREE.Texture {
  const size = 128;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (ctx) {
    const image = ctx.createImageData(size, size);
    // Deterministic, so the model looks the same on every launch.
    let seed = 0x2f6e2b1;
    for (let i = 0; i < size * size; i++) {
      seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
      const v = 224 + (seed >>> 27);
      image.data[i * 4] = v;
      image.data[i * 4 + 1] = v;
      image.data[i * 4 + 2] = v;
      image.data[i * 4 + 3] = 255;
    }
    ctx.putImageData(image, 0, 0);
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.NoColorSpace;
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  return texture;
}
