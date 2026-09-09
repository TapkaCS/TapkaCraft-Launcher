// Generates TapkaCraft's original brand assets: the app icon (in the sizes
// Tauri's bundler needs, plus a hand-rolled .ico) and a tileable pixel "dirt"
// background texture for the retro theme.
//
// Everything here is procedural pixel art built from a small deterministic
// palette - no Mojang/Minecraft assets are read, copied, or referenced.
//
// Run with: npm run gen:assets
import { PNG } from "pngjs";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

/** @param {string} hex */
function hexToRgb(hex) {
  const n = parseInt(hex.replace("#", ""), 16);
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}

/**
 * Builds a PNG buffer from a 2D grid of hex color strings (or null for
 * transparent), scaling each grid cell up to `cellSize` device pixels so the
 * result reads as chunky pixel art rather than a 1:1 thumbnail.
 * @param {(string | null)[][]} grid
 * @param {number} cellSize
 */
function renderGrid(grid, cellSize) {
  const rows = grid.length;
  const cols = grid[0].length;
  const png = new PNG({ width: cols * cellSize, height: rows * cellSize });

  for (let gy = 0; gy < rows; gy++) {
    for (let gx = 0; gx < cols; gx++) {
      const color = grid[gy][gx];
      const rgba = color ? { ...hexToRgb(color), a: 255 } : { r: 0, g: 0, b: 0, a: 0 };
      for (let py = 0; py < cellSize; py++) {
        for (let px = 0; px < cellSize; px++) {
          const x = gx * cellSize + px;
          const y = gy * cellSize + py;
          const idx = (cols * cellSize * y + x) << 2;
          png.data[idx] = rgba.r;
          png.data[idx + 1] = rgba.g;
          png.data[idx + 2] = rgba.b;
          png.data[idx + 3] = rgba.a;
        }
      }
    }
  }
  return PNG.sync.write(png);
}

// --- deterministic PRNG (mulberry32) so regenerating assets is reproducible ---
function mulberry32(seed) {
  let a = seed;
  return function () {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// ============================================================================
// App icon: a beveled "slot" block with a bold pixel T monogram over a thin
// dirt-toned footer strip. 16x16 logical grid.
// ============================================================================
function buildIconGrid() {
  const SIZE = 16;
  const palette = {
    border: "#241a12",
    ring: "#2f5417",
    shade: "#3c6b1f",
    mid: "#57a12c",
    light: "#7fd646",
    glyph: "#f2ead2",
    dirt: "#6b4423",
    dirtDark: "#55341a",
  };

  const isTGlyph = (x, y) => (y >= 3 && y <= 4 && x >= 4 && x <= 11) || (y >= 5 && y <= 9 && x >= 7 && x <= 8);

  /** @type {(string | null)[][]} */
  const grid = [];
  for (let y = 0; y < SIZE; y++) {
    const row = [];
    for (let x = 0; x < SIZE; x++) {
      const edge = x === 0 || y === 0 || x === SIZE - 1 || y === SIZE - 1;
      const ring = x === 1 || y === 1 || x === SIZE - 2 || y === SIZE - 2;
      if (edge) {
        row.push(palette.border);
        continue;
      }
      if (ring) {
        row.push(palette.ring);
        continue;
      }
      const isDirtStrip = y >= 11 && y <= 13;
      if (isDirtStrip) {
        row.push((x * 7 + y * 13) % 5 === 0 ? palette.dirtDark : palette.dirt);
        continue;
      }
      if (isTGlyph(x, y)) {
        row.push(palette.glyph);
        continue;
      }
      const diagonal = x - y;
      if (diagonal > 2) row.push(palette.shade);
      else if (diagonal < -2) row.push(palette.light);
      else row.push(palette.mid);
    }
    grid.push(row);
  }
  return grid;
}

// ============================================================================
// Dirt background texture: chunky procedural speckle over an 8x8 logical
// grid (each logical cell rendered as 2x2 raw pixels -> 16x16 tile), so it
// stays tileable via CSS background-repeat and looks like deliberate pixel
// art rather than smooth noise once scaled with image-rendering: pixelated.
// ============================================================================
function buildDirtTile() {
  const LOGICAL = 8;
  const CELL_PIXELS = 2; // -> 16x16 PNG tile
  const rand = mulberry32(0xd12707);
  const palette = ["#6b4a2f", "#5a3d26", "#7d5a3a", "#4a3220", "#8a6844"];
  const weights = [0.52, 0.2, 0.16, 0.06, 0.06];

  const pick = () => {
    const r = rand();
    let acc = 0;
    for (let i = 0; i < palette.length; i++) {
      acc += weights[i];
      if (r <= acc) return palette[i];
    }
    return palette[0];
  };

  /** @type {string[][]} */
  const logical = Array.from({ length: LOGICAL }, () => Array.from({ length: LOGICAL }, pick));

  /** @type {string[][]} */
  const grid = [];
  for (let y = 0; y < LOGICAL; y++) {
    for (let sub = 0; sub < 1; sub++) {
      const row = [];
      for (let x = 0; x < LOGICAL; x++) row.push(logical[y][x]);
      grid.push(row);
    }
  }
  return renderGrid(grid, CELL_PIXELS);
}

// --- minimal ICO container (Vista+ PNG-in-ICO entries) ---
/** @param {{ size: number; png: Buffer }[]} images */
function buildIco(images) {
  const headerSize = 6 + images.length * 16;
  let offset = headerSize;
  const header = Buffer.alloc(headerSize);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // type: icon
  header.writeUInt16LE(images.length, 4);

  images.forEach((img, i) => {
    const entryOffset = 6 + i * 16;
    const dim = img.size >= 256 ? 0 : img.size; // 0 means 256 per ICO spec
    header.writeUInt8(dim, entryOffset + 0); // width
    header.writeUInt8(dim, entryOffset + 1); // height
    header.writeUInt8(0, entryOffset + 2); // color count
    header.writeUInt8(0, entryOffset + 3); // reserved
    header.writeUInt16LE(1, entryOffset + 4); // planes
    header.writeUInt16LE(32, entryOffset + 6); // bit count
    header.writeUInt32LE(img.png.length, entryOffset + 8); // bytes in resource
    header.writeUInt32LE(offset, entryOffset + 12); // image offset
    offset += img.png.length;
  });

  return Buffer.concat([header, ...images.map((i) => i.png)]);
}

function main() {
  const iconGrid = buildIconGrid();
  const iconDir = path.join(root, "src-tauri", "icons");
  mkdirSync(iconDir, { recursive: true });

  const sizes = [16, 32, 48, 128, 256, 512];
  /** @type {Record<number, Buffer>} */
  const rendered = {};
  for (const size of sizes) {
    rendered[size] = renderGrid(iconGrid, size / 16);
  }

  writeFileSync(path.join(iconDir, "32x32.png"), rendered[32]);
  writeFileSync(path.join(iconDir, "128x128.png"), rendered[128]);
  writeFileSync(path.join(iconDir, "128x128@2x.png"), rendered[256]);
  writeFileSync(path.join(iconDir, "icon.png"), rendered[512]);
  writeFileSync(
    path.join(iconDir, "icon.ico"),
    buildIco([16, 32, 48, 256].map((size) => ({ size, png: rendered[size] }))),
  );
  console.log(`icons written to ${path.relative(root, iconDir)}/`);

  const textureDir = path.join(root, "src", "assets", "textures");
  mkdirSync(textureDir, { recursive: true });
  writeFileSync(path.join(textureDir, "dirt.png"), buildDirtTile());
  console.log(`dirt texture written to ${path.relative(root, textureDir)}/dirt.png`);

  const brandDir = path.join(root, "src", "assets", "brand");
  mkdirSync(brandDir, { recursive: true });
  writeFileSync(path.join(brandDir, "mark.png"), rendered[256]);
  console.log(`in-app brand mark written to ${path.relative(root, brandDir)}/mark.png`);
}

main();
