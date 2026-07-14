import { execSync } from "node:child_process";
import { cpSync, rmSync, existsSync } from "node:fs";
import { resolve, join, delimiter } from "node:path";
import { platform } from "node:process";

const root = resolve(import.meta.dirname, "..");
const pythonDir = join(root, "src-python");
const vueDist = join(root, "dist");
const staticDir = join(pythonDir, "static");
const pyDistDir = join(pythonDir, "dist", "fstdict-server");
const sidecarTargetDir = join(root, "src-tauri", "sidecars", "fstdict-server");

// PyInstaller --add-data separator: colon on Unix, semicolon on Windows
const sep = platform === "win32" ? ";" : ":";

console.log("\n═══ Building Python sidecar ═══\n");

// 1. Clean old static directory
if (existsSync(staticDir)) {
    rmSync(staticDir, { recursive: true });
}

// 2. Copy Vue build output to Python static/
cpSync(vueDist, staticDir, { recursive: true });
console.log("  ✓ Vue dist → src-python/static");

// 3. Run PyInstaller
console.log("  Running PyInstaller...");
const addDataStatic = `static${sep}static`;
const addDataConfig = `config.json${sep}.`;

execSync(
    `pyinstaller --clean -y --onedir --noconsole --name fstdict-server ` +
        `--add-data "${addDataStatic}" --add-data "${addDataConfig}" ` +
        `fstdict-server.py`,
    { cwd: pythonDir, stdio: "inherit" },
);

// 4. Clean old sidecar in Tauri
if (existsSync(sidecarTargetDir)) {
    rmSync(sidecarTargetDir, { recursive: true });
}

// 5. Copy PyInstaller output to sidecars/
cpSync(pyDistDir, sidecarTargetDir, { recursive: true });
console.log(`  ✓ Sidecar → src-tauri/sidecars/fstdict-server/`);

console.log("\n✓ Python sidecar build complete\n");
