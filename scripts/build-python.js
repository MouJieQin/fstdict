import { execSync } from "node:child_process";
import { cpSync, rmSync, existsSync, mkdirSync, renameSync } from "node:fs";
import { resolve, join } from "node:path";
import { platform, arch } from "node:process";

// === 1. FFmpeg 核心版本配置 ===
const FFMPEG_VERSION_TAG = "v8.1.2-build3";
const REPO_URL = `https://github.com/MouJieQin/fstdict-ffmpeg/releases/download/${FFMPEG_VERSION_TAG}`;

const root = resolve(import.meta.dirname, "..");
const pythonDir = join(root, "src-python");
const vueDist = join(root, "dist");
const staticDir = join(pythonDir, "static");
const pyDistDir = join(pythonDir, "dist", "fstdict-server");
const sidecarTargetDir = join(root, "src-tauri", "sidecars", "fstdict-server");

const sep = platform === "win32" ? ";" : ":";

console.log("\n═══ Building Python sidecar ═══\n");

// === 2. 交叉编译架构识别核心机制 (基于编译目标而非宿主机环境) ===
const sysPlatform = platform;
const rustTarget = process.env.TAURI_TARGET || "";

// 默认采用当前主机的架构属性
let targetArch = arch;

// ✨ 修复核心：如果当前在 MacOS 且检测到显式指定的跨平台 Target 编译参数，强制纠正目标架构
if (sysPlatform === "darwin") {
    if (rustTarget.includes("x86_64") || process.argv.includes("x86_64")) {
        targetArch = "x86_64";
    } else if (
        rustTarget.includes("aarch64") ||
        process.argv.includes("aarch64") ||
        rustTarget.includes("arm64")
    ) {
        targetArch = "arm64";
    }
}

// === 3. 检查、下载并解压对应平台的 FFmpeg 二进制文件 ===
const ffmpegDir = join(pythonDir, "ffmpeg");
const ffmpegExecutableName =
    sysPlatform === "win32" ? "fstdict-ffmpeg.exe" : "fstdict-ffmpeg";
const ffmpegLocalPath = join(ffmpegDir, ffmpegExecutableName);

if (!existsSync(ffmpegLocalPath)) {
    console.log(
        `  ⚠ Local FFmpeg binary not found at ${ffmpegLocalPath}. Target Arch: [${targetArch}]. Starting download...`,
    );

    let archiveName = "";
    let innerFileName = "";

    if (sysPlatform === "darwin") {
        // ✨ 使用纠正后的 targetArch 代替原本的机器宿主 sysArch
        if (targetArch === "arm64") {
            archiveName = "ffmpeg-macos-arm64.tar.gz";
            innerFileName = "ffmpeg-macos-arm64";
        } else {
            archiveName = "ffmpeg-macos-x86_64.tar.gz";
            innerFileName = "ffmpeg-macos-x86_64";
        }
    } else if (sysPlatform === "linux") {
        archiveName = "ffmpeg-linux-x86_64.tar.gz";
        innerFileName = "ffmpeg-linux-x86_64";
    } else if (sysPlatform === "win32") {
        archiveName = "ffmpeg-windows-x86_64.zip";
        innerFileName = "ffmpeg-windows-x86_64.exe";
    } else {
        throw new Error(`Unsupported platform framework: ${sysPlatform}`);
    }

    const downloadUrl = `${REPO_URL}/${archiveName}`;
    const archivePath = join(pythonDir, archiveName);

    if (!existsSync(ffmpegDir)) {
        mkdirSync(ffmpegDir, { recursive: true });
    }

    try {
        console.log(`  Downloading ${downloadUrl} ...`);
        execSync(`curl -L -o "${archivePath}" "${downloadUrl}"`, {
            stdio: "inherit",
        });

        console.log(`  Extracting ${archiveName} ...`);
        if (archiveName.endsWith(".tar.gz")) {
            execSync(`tar -xzf "${archivePath}" -C "${ffmpegDir}"`, {
                stdio: "inherit",
            });

            const genericUnpackedFile = join(ffmpegDir, innerFileName);
            if (existsSync(genericUnpackedFile)) {
                renameSync(genericUnpackedFile, ffmpegLocalPath);
            }
        } else if (archiveName.endsWith(".zip")) {
            execSync(
                `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${ffmpegDir}' -Force"`,
                { stdio: "inherit" },
            );

            const genericUnpackedExe = join(ffmpegDir, innerFileName);
            if (existsSync(genericUnpackedExe)) {
                renameSync(genericUnpackedExe, ffmpegLocalPath);
            }
        }

        if (existsSync(archivePath)) {
            rmSync(archivePath);
        }

        if (sysPlatform !== "win32") {
            execSync(`chmod +x "${ffmpegLocalPath}"`);
        }

        console.log(`  ✓ FFmpeg setup successful: ${ffmpegLocalPath}`);
    } catch (error) {
        throw new Error(
            `Failed to automatically fetch FFmpeg asset: ${error.message}`,
        );
    }
} else {
    console.log(
        `  ✓ FFmpeg binary already exists locally at: ${ffmpegLocalPath}`,
    );
}

// 4. Clean old static directory
if (existsSync(staticDir)) {
    rmSync(staticDir, { recursive: true });
}

// 5. Copy Vue build output to Python static/
cpSync(vueDist, staticDir, { recursive: true });
console.log("  ✓ Vue dist → src-python/static");

// 6. Run PyInstaller
console.log("  Running PyInstaller...");
const addDataStatic = `static${sep}static`;
const addDataConfig = `config.json${sep}.`;
const addDataCgeventConfig = `cgevent_config.json${sep}.`;
const addDataFfmpeg = `ffmpeg${sep}ffmpeg`;

let commandPrefix = "";
let targetArchFlag = "";
const isMac = sysPlatform === "darwin";

if (isMac) {
    if (targetArch === "x86_64") {
        commandPrefix = "arch -x86_64 ";
        targetArchFlag = " --target-arch x86_64";
    } else if (targetArch === "arm64") {
        targetArchFlag = " --target-arch arm64";
    }
}

execSync(
    `${commandPrefix}pyinstaller --clean -y --onedir --noconsole --name fstdict-server ` +
        `--add-data "${addDataStatic}" --add-data "${addDataConfig}" --add-data "${addDataCgeventConfig}" --add-data "${addDataFfmpeg}"${targetArchFlag} ` +
        `fstdict-server.py`,
    { cwd: pythonDir, stdio: "inherit" },
);

// 7. Clean old sidecar in Tauri
if (existsSync(sidecarTargetDir)) {
    rmSync(sidecarTargetDir, { recursive: true });
}

// 8. Copy PyInstaller output to sidecars/
cpSync(pyDistDir, sidecarTargetDir, { recursive: true });
console.log(`  ✓ Sidecar → src-tauri/sidecars/fstdict-server/`);

console.log("\n✓ Python sidecar build complete\n");
