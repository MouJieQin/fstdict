import { execSync } from "node:child_process";
import { platform } from "node:process";
import { resolve, join } from "node:path";
import { mkdirSync, copyFileSync, existsSync } from "node:fs";

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

// Helper to get the current Rust target triple (e.g., aarch64-apple-darwin)
const getHostTriple = () => {
    return execSync("rustc -vV")
        .toString()
        .match(/host: (\S+)/)?.[1];
};

try {
    if (isRelease) {
        console.log("\n[Step] Build Python fstdict-server");
        runCommand(`node ${join(root, "scripts/build-python.js")}`);
    } else {
        console.log("[Skip] Python sidecar (dev mode)");
    }

    if (isDarwin) {
        console.log("\n[Step] Build C++ helper fstdict_cgevent_server");
        const cppScript = join(root, "scripts/build-helper.js");
        runCommand(`node ${cppScript} ${isRelease ? "--release" : ""}`);

        console.log("\n[Step] Build Rust fstdict-helper (Isolated Package)");
        const cargoFlag = isRelease ? "--release" : "";
        const rustTarget = process.env.TAURI_TARGET || "";
        const targetArg = rustTarget ? `--target ${rustTarget}` : "";

        // CHANGED: Use "-p fstdict-helper" to build the isolated package
        // No need for TAURI_CONFIG_PATH anymore!
        runCommand(
            `cargo build -p fstdict-helper ${cargoFlag} ${targetArg} --manifest-path src-tauri/Cargo.toml`,
        );

        // 2. Prepare the binaries directory
        const binariesDir = join(root, "src-tauri/binaries");
        mkdirSync(binariesDir, { recursive: true });

        // 3. Identify paths
        const targetTriple = process.env.TAURI_TARGET || getHostTriple();
        const buildType = isRelease ? "release" : "debug";

        // Source: Where Cargo put the compiled binary
        const sourceBin = join(
            root,
            `src-tauri/target/${buildType}/fstdict-helper`,
        );

        // Dest: Where Tauri Bundler expects it (MUST include target triple)
        const destBin = join(binariesDir, `fstdict-helper-${targetTriple}`);

        // 4. Copy and Rename
        if (existsSync(sourceBin)) {
            console.log(`\n[Sidecar] Copying binary to: ${destBin}`);
            copyFileSync(sourceBin, destBin);
        } else {
            throw new Error(`Binary not found at ${sourceBin}`);
        }
        console.log("  ✓ Rust fstdict-helper bundled successfully\n");
    } else {
        console.log("\nℹ Skip macOS-only binaries (not darwin)");
    }
    console.log("\n✅ All sidecar tasks completed\n");
} catch (err) {
    console.error("\n❌ Build failed", err.message);
    process.exit(1);
}
