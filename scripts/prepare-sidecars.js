import { chmodSync, copyFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import ffmpegPath from "ffmpeg-static";

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, "..");
const targetTriple = process.env.TAURI_TARGET_TRIPLE ?? hostTriple();
const targetPath = resolve(projectRoot, "src-tauri", "binaries", `ffmpeg-${targetTriple}`);
const binaryDir = dirname(targetPath);

if (!ffmpegPath || !existsSync(ffmpegPath)) {
  throw new Error("ffmpeg-static did not provide a local ffmpeg binary.");
}

mkdirSync(binaryDir, { recursive: true });
copyFileSync(ffmpegPath, targetPath);
chmodSync(targetPath, 0o755);
copyNotice("ffmpeg.LICENSE");
copyNotice("ffmpeg.README");

const bytes = statSync(targetPath).size;
console.log(`Prepared ffmpeg sidecar: ${targetPath} (${bytes} bytes)`);

function copyNotice(fileName) {
  const source = join(dirname(ffmpegPath), fileName);
  if (existsSync(source)) {
    copyFileSync(source, join(binaryDir, fileName));
  }
}

function hostTriple() {
  if (process.platform === "darwin" && process.arch === "arm64") {
    return "aarch64-apple-darwin";
  }
  if (process.platform === "darwin" && process.arch === "x64") {
    return "x86_64-apple-darwin";
  }
  throw new Error(`Unsupported sidecar host: ${process.platform}/${process.arch}`);
}
