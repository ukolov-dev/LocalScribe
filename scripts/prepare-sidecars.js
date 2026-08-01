import { chmodSync, existsSync, statSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, "..");
const targetTriple =
  process.env.TAURI_TARGET_TRIPLE ?? process.env.TAURI_ENV_TARGET_TRIPLE ?? hostTriple();
const targetPath = resolve(projectRoot, "src-tauri", "binaries", `ffmpeg-${targetTriple}`);
const buildScript = resolve(projectRoot, "scripts", "build-ffmpeg-lgpl.sh");

if (!isExpectedBuild(targetPath)) {
  execFileSync(buildScript, [], {
    cwd: projectRoot,
    env: { ...process.env, TAURI_TARGET_TRIPLE: targetTriple },
    stdio: "inherit",
  });
}

chmodSync(targetPath, 0o755);

const bytes = statSync(targetPath).size;
console.log(`Verified LGPL-only ffmpeg sidecar: ${targetPath} (${bytes} bytes)`);

function isExpectedBuild(binaryPath) {
  if (!existsSync(binaryPath)) {
    return false;
  }

  try {
    const version = execFileSync(binaryPath, ["-version"], { encoding: "utf8" });
    return (
      version.includes("ffmpeg version 8.1.2") &&
      !version.includes("--enable-gpl") &&
      !version.includes("--enable-nonfree")
    );
  } catch {
    return false;
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
