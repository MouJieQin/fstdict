import { execSync } from "node:child_process";
import { platform } from "node:process";
import { resolve, join } from "node:path";
import { mkdirSync, copyFileSync, writeFileSync } from "node:fs";

const root = resolve(import.meta.dirname, "..");
const isDarwin = platform === "darwin";
const isRelease = process.argv.includes("--release");

console.log("\n========== Sidecar Build Dispatcher ==========\n");

// MODIFIED: Added an optional env parameter to the helper function
const runCommand = (cmd, extraEnv = {}) => {
    console.log(`> ${cmd}`);
    execSync(cmd, {
        stdio: "inherit",
        cwd: root,
    });
};

// Helper: Get the exact target triple (e.g., x86_64-pc-windows-msvc)
const getTargetTriple = () => {
    // Use the explicit flag if provided, otherwise ask Rust
    if (process.env.TAURI_TARGET) return process.env.TAURI_TARGET;
    try {
        return execSync("rustc -vV")
            .toString()
            .match(/host: (\S+)/)[1];
    } catch {
        // Fallback for standard architectures if rustc fails
        return platform === "win32"
            ? "x86_64-pc-windows-msvc"
            : "x86_64-unknown-linux-gnu";
    }
};

try {
    if (isRelease) {
        console.log("\n[Step] Build Python fstdict-server");
        runCommand(`node ${join(root, "scripts/build-python.js")}`);
    } else {
        console.log("[Skip] Python sidecar (dev mode)");
    }

    // 1. Ensure binaries folder exists
    const binariesDir = join(root, "src-tauri/binaries");
    mkdirSync(binariesDir, { recursive: true });

    // 2. Calculate the specific filename Tauri expects
    const triple = getTargetTriple();
    const extension = platform === "win32" ? ".exe" : "";
    const sidecarName = `fstdict-helper-${triple}${extension}`;
    const sidecarPath = join(binariesDir, sidecarName);

    // ============================================================
    // STRATEGY: Real Compilation (macOS) vs. Dummy File (Others)
    // ============================================================

    if (isDarwin) {
        console.log("\n[Step] Build C++ helper fstdict_cgevent_server");
        const cppScript = join(root, "scripts/build-helper.js");
        runCommand(`node ${cppScript} ${isRelease ? "--release" : ""}`);

        console.log(
            `\n🍎 [macOS] Compiling real fstdict-helper (Isolated Package) binary...`,
        );
        // Build the helper package
        const cargoFlag = isRelease ? "--release" : "";
        const buildType = isRelease ? "release" : "debug";
        const rustTarget = process.env.TAURI_TARGET || "";
        const targetArg = rustTarget ? `--target ${rustTarget}` : "";

        // CHANGED: Use "-p fstdict-helper" to build the isolated package
        // No need for TAURI_CONFIG_PATH anymore!
        runCommand(
            `cargo build -p fstdict-helper ${cargoFlag} ${targetArg} --manifest-path src-tauri/Cargo.toml`,
            {
                stdio: "inherit",
                cwd: root,
            },
        );

        // 2. Prepare the binaries directory
        const binariesDir = join(root, "src-tauri/binaries");
        mkdirSync(binariesDir, { recursive: true });

        // Copy the real binary to the target location
        const sourceBin = join(
            root,
            `src-tauri/target/${buildType}/fstdict-helper`,
        );
        copyFileSync(sourceBin, sidecarPath);
        console.log(`✅ Bundled real binary: ${sidecarName}`);
    } else {
        console.log(`\n🐧/🪟 [Win/Linux] Creating DUMMY Helper binary...`);
        // Create an empty file to satisfy Tauri's bundler check
        writeFileSync(sidecarPath, "DUMMY_CONTENT_FOR_TAURI_BUNDLE");
        console.log(`⚠️  Bundled dummy file: ${sidecarName}`);
    }
    console.log("\n✅ All sidecar tasks completed\n");
} catch (err) {
    console.error("\n❌ Build failed", err.message);
    process.exit(1);
}
