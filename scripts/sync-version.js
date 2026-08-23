import { readFileSync, writeFileSync } from "node:fs";
import { resolve, join } from "node:path";

const root = resolve(import.meta.dirname, "..");
const version = readFileSync(join(root, "VERSION"), "utf8").trim();

// ── Update package.json ──
const pkgPath = join(root, "package.json");
const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
pkg.version = version;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
console.log(`  ✓ package.json → ${version}`);

// ── Update tauri.conf.json ──
const tauriPath = join(root, "src-tauri", "tauri.conf.json");
const tauriConf = JSON.parse(readFileSync(tauriPath, "utf8"));
tauriConf.version = version;
writeFileSync(tauriPath, JSON.stringify(tauriConf, null, 2) + "\n");
console.log(`  ✓ tauri.conf.json → ${version}`);

// ── Update helper/tauri.conf.json ──
const helperTauriPath = join(root, "src-tauri", "helper", "tauri.conf.json");
const helperTauriConf = JSON.parse(readFileSync(helperTauriPath, "utf8"));
helperTauriConf.version = version;
writeFileSync(helperTauriPath, JSON.stringify(helperTauriConf, null, 2) + "\n");
console.log(`  ✓ helper/tauri.conf.json → ${version}`);

// ── Update Cargo.toml ──
const cargoPath = join(root, "src-tauri", "Cargo.toml");
let cargo = readFileSync(cargoPath, "utf8");
cargo = cargo.replace(/^version\s*=\s*".*"/m, `version = "${version}"`);
writeFileSync(cargoPath, cargo);
console.log(`  ✓ Cargo.toml → ${version}`);

// ── Update helper/Cargo.toml ──
const helperCargoPath = join(root, "src-tauri", "helper", "Cargo.toml");
let helperCargo = readFileSync(helperCargoPath, "utf8");
helperCargo = helperCargo.replace(
    /^version\s*=\s*".*"/m,
    `version = "${version}"`,
);
writeFileSync(helperCargoPath, helperCargo);
console.log(`  ✓ helper/Cargo.toml → ${version}`);

console.log(`\nAll version files synced to ${version}`);
