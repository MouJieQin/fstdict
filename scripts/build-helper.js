import { execSync } from "node:child_process";
import { cpSync, rmSync, existsSync, mkdirSync } from "node:fs";
import { resolve, join } from "node:path";
import { platform } from "node:process";

const root = resolve(import.meta.dirname, "..");
const cppProjectDir = join(root, "src-helper", "fstdict-cgevent-server");
const buildDir = join(cppProjectDir, "build");
const binSourcePath = join(buildDir, "bin", "fstdict_cgevent_server");
const sidecarTargetDir = join(root, "src-tauri", "sidecars", "helper");

// 传入参数 --release 代表正式打包，清空构建目录；dev模式增量编译
const isReleaseBuild = process.argv.includes("--release");

console.log("\n═══ Building Helper C++ Sidecar (macOS only) ═══\n");

if (platform !== "darwin") {
    console.log("  ℹ Skip build: current platform is not macOS");
    process.exit(0);
}

const rustTarget = process.env.TAURI_TARGET || "";
let cmakeArchFlag = "";
let buildArch = "native";

if (rustTarget.includes("x86_64-apple-darwin")) {
    buildArch = "x86_64";
    cmakeArchFlag = "-DCMAKE_OSX_ARCHITECTURES=x86_64";
} else if (rustTarget.includes("aarch64-apple-darwin")) {
    buildArch = "arm64";
    cmakeArchFlag = "-DCMAKE_OSX_ARCHITECTURES=arm64";
}
console.log(`  Target arch: ${buildArch}`);

// ============ 增量编译逻辑 ============
if (isReleaseBuild) {
    // Release打包：全量清理
    if (existsSync(buildDir)) {
        rmSync(buildDir, { recursive: true });
    }
    mkdirSync(buildDir, { recursive: true });
}
// Dev模式：不删除build目录，直接复用缓存增量编译

// CI环境自动开启FETCH_DEPS静态链接，本地开发使用brew
const fetchFlag = !!process.env.CI ? "-DFETCH_DEPS=ON" : "";
const buildType = isReleaseBuild ? "Release" : "Debug";

// configure仅在build目录不存在时执行（增量优化）
if (!existsSync(join(buildDir, "CMakeCache.txt"))) {
    console.log("  Running CMake configure...");
    execSync(
        `cmake -B build -DCMAKE_BUILD_TYPE=${buildType} ${fetchFlag} ${cmakeArchFlag}`,
        { cwd: cppProjectDir, stdio: "inherit" },
    );
}

console.log("  Running CMake build...");
execSync(`cmake --build build -j$(sysctl -n hw.ncpu)`, {
    cwd: cppProjectDir,
    stdio: "inherit",
});

// 同步二进制到sidecar
if (existsSync(sidecarTargetDir)) {
    rmSync(sidecarTargetDir, { recursive: true });
}
mkdirSync(sidecarTargetDir, { recursive: true });
const targetBinPath = join(sidecarTargetDir, "fstdict_cgevent_server");
cpSync(binSourcePath, targetBinPath);
console.log(`  ✓ Binary copied to ${targetBinPath}`);

console.log("\n✓ Helper sidecar build complete\n");
