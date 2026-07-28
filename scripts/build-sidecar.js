import { execSync } from "node:child_process";
import { platform } from "node:process";
import { resolve, join } from "node:path";

const root = resolve(import.meta.dirname, "..");
const isDarwin = platform === "darwin";
// 入参识别模式
const isRelease = process.argv.includes("--release");
const isDev = process.argv.includes("--dev");

console.log("\n========== Sidecar Build Dispatcher ==========\n");

const runCommand = (cmd) => {
  console.log(`> ${cmd}`);
  execSync(cmd, { stdio: "inherit", cwd: root });
};

try {
  // ========== Python sidecar：BUILD模式才执行，DEV模式跳过 ==========
  if (isRelease) {
    console.log("\n[Step] Build Python fstdict-server");
    runCommand(`node ${join(root, "scripts/build-python.js")}`);
  } else {
    console.log("[Skip] Python sidecar (dev mode)");
  }

  // ========== macOS专属任务：dev & build 都执行；非mac直接跳过 ==========
  if (isDarwin) {
    console.log("\n[Step] Build C++ helper fstdict_cgevent_server");
    const cppScript = join(root, "scripts/build-helper.js");
    runCommand(`node ${cppScript} ${isRelease ? "--release" : ""}`);

    console.log("\n[Step] Build Rust fstdict-helper");
    const cargoFlag = isRelease ? "--release" : "";
    runCommand(`cargo build --bin fstdict-helper ${cargoFlag} --manifest-path src-tauri/Cargo.toml`);

    // 将rust输出二进制复制到sidecar目录（关键！）
    const fs = await import("node:fs");
    const srcBin = join(root, "src-tauri", "target", isRelease ? "release" : "debug", "fstdict-helper");
    const destDir = join(root, "src-tauri", "sidecars", "helper");
    if (fs.existsSync(destDir)) fs.rmSync(destDir, { recursive: true });
    fs.mkdirSync(destDir, { recursive: true });
    fs.cpSync(srcBin, join(destDir, "fstdict-helper"));
    console.log(`  ✓ Rust fstdict-helper copied to sidecars/helper/`);
  } else {
    console.log("\nℹ Skip macOS-only binaries (not darwin)");
  }

  console.log("\n✅ All sidecar tasks completed\n");
} catch (err) {
  console.error("\n❌ Build failed", err.message);
  process.exit(1);
}