import { execSync } from "node:child_process";
import { resolve, join } from "node:path";

const root = resolve(import.meta.dirname, "..");
const TauriDir = join(root, "src-tauri");

console.log("  Running cargo clean...");
execSync(`cargo clean`, {
    cwd: TauriDir,
    stdio: "inherit",
});
