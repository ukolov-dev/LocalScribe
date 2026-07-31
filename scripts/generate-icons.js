import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { deflateSync } from "node:zlib";

const iconDir = resolve("src-tauri", "icons");
mkdirSync(iconDir, { recursive: true });

for (const [fileName, size] of [
  ["32x32.png", 32],
  ["128x128.png", 128],
  ["128x128@2x.png", 256],
  ["icon.png", 512]
]) {
  writeFileSync(resolve(iconDir, fileName), renderIcon(size));
}

console.log(`Generated LocalScribe icon set in ${iconDir}`);

function renderIcon(size) {
  const data = Buffer.alloc((size * 4 + 1) * size);
  let offset = 0;

  for (let y = 0; y < size; y += 1) {
    data[offset++] = 0;
    for (let x = 0; x < size; x += 1) {
      const [r, g, b, a] = pixel(size, x, y);
      data[offset++] = r;
      data[offset++] = g;
      data[offset++] = b;
      data[offset++] = a;
    }
  }

  return concat(
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", concat(u32(size), u32(size), Buffer.from([8, 6, 0, 0, 0]))),
    chunk("IDAT", deflateSync(data, { level: 9 })),
    chunk("IEND", Buffer.alloc(0))
  );
}

function pixel(size, x, y) {
  const cx = size / 2;
  const cy = size / 2;
  const dx = (x - cx) / size;
  const dy = (y - cy) / size;
  const radius = Math.sqrt(dx * dx + dy * dy);
  const alpha = roundedSquareAlpha(size, x, y, size * 0.18);

  if (alpha === 0) {
    return [0, 0, 0, 0];
  }

  const t = y / Math.max(1, size - 1);
  let r = mix(25, 47, t);
  let g = mix(44, 111, t);
  let b = mix(38, 99, t);

  const lineY = size * 0.55 + Math.sin((x / size) * Math.PI * 4.5) * size * 0.08;
  const waveform = Math.abs(y - lineY) < size * 0.025;
  const cursor = Math.abs(x - cx) < size * 0.065 && y > size * 0.22 && y < size * 0.78;
  const halo = Math.abs(radius - 0.18) < 0.018 && y < cy;

  if (waveform || cursor || halo) {
    r = 239;
    g = 246;
    b = 224;
  }

  return [r, g, b, Math.round(255 * alpha)];
}

function roundedSquareAlpha(size, x, y, radius) {
  const min = radius;
  const max = size - radius - 1;
  const px = Math.max(min, Math.min(max, x));
  const py = Math.max(min, Math.min(max, y));
  const distance = Math.hypot(x - px, y - py);
  if (distance <= radius - 1) {
    return 1;
  }
  if (distance >= radius) {
    return 0;
  }
  return radius - distance;
}

function mix(a, b, t) {
  return Math.round(a + (b - a) * t);
}

function u32(value) {
  const buffer = Buffer.alloc(4);
  buffer.writeUInt32BE(value);
  return buffer;
}

function chunk(type, data) {
  const typeBuffer = Buffer.from(type);
  return concat(u32(data.length), typeBuffer, data, u32(crc32(concat(typeBuffer, data))));
}

function concat(...buffers) {
  return Buffer.concat(buffers);
}

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}
