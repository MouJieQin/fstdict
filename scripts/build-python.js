import { execSync } from "node:child_process";
import { cpSync, rmSync, existsSync, mkdirSync, renameSync } from "node:fs";
import { resolve, join } from "node:path";
import { platform, arch } from "node:process"; // process.arch is safer than os.arch()

// === 1. FFmpeg 核心版本配置 (使用 /download/ 确保拿到真实直链) ===
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

// === 2. 检查、下载并解压对应平台的 FFmpeg 二进制文件 ===
const ffmpegDir = join(pythonDir, "ffmpeg");
// 运行时的期望名称 (与 Python 代码中的调用保持一致)
const ffmpegExecutableName =
    platform === "win32" ? "fstdict-ffmpeg.exe" : "fstdict-ffmpeg";
const ffmpegLocalPath = join(ffmpegDir, ffmpegExecutableName);

if (!existsSync(ffmpegLocalPath)) {
    console.log(
        `  ⚠ Local FFmpeg binary not found at ${ffmpegLocalPath}. Starting download...`,
    );

    let archiveName = "";
    let innerFileName = ""; // 解压后原本包含的文件名

    const sysPlatform = platform;
    const sysArch = arch; // 直接读取字符串，避免全局 arch() 冲突

    if (sysPlatform === "darwin") {
        if (sysArch === "arm64") {
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
            // 解压到指定的临时存放文件夹
            execSync(`tar -xzf "${archivePath}" -C "${ffmpegDir}"`, {
                stdio: "inherit",
            });

            // 修复：读取你在 Release 包中实际保留的特定平台专属名称并重命名
            const genericUnpackedFile = join(ffmpegDir, innerFileName);
            if (existsSync(genericUnpackedFile)) {
                renameSync(genericUnpackedFile, ffmpegLocalPath);
            }
        } else if (archiveName.endsWith(".zip")) {
            execSync(
                `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${ffmpegDir}' -Force"`,
                { stdio: "inherit" },
            );

            // 修复：针对 Windows 解压出的特定的平台名 "ffmpeg-windows-x86_64.exe" 转换为期望名称
            const genericUnpackedExe = join(ffmpegDir, innerFileName);
            if (existsSync(genericUnpackedExe)) {
                renameSync(genericUnpackedExe, ffmpegLocalPath);
            }
        }

        // 清理临时下载的压缩包
        if (existsSync(archivePath)) {
            rmSync(archivePath);
        }

        // 赋予非 Windows 系统可执行权限，确保侧载正常
        if (platform !== "win32") {
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

// 3. Clean old static directory
if (existsSync(staticDir)) {
    rmSync(staticDir, { recursive: true });
}

// 4. Copy Vue build output to Python static/
cpSync(vueDist, staticDir, { recursive: true });
console.log("  ✓ Vue dist → src-python/static");

// 5. Run PyInstaller
console.log("  Running PyInstaller...");
const addDataStatic = `static${sep}static`;
const addDataConfig = `config.json${sep}.`;
const addDataCgeventConfig = `cgevent_config.json${sep}.`;

// 映射整个 ffmpeg 文件夹，这样 PyInstaller 会在解压时保持内部的 fstdict-ffmpeg 路径
const addDataFfmpeg = `ffmpeg${sep}ffmpeg`;

let commandPrefix = "";
let targetArchFlag = "";
const isMac = platform === "darwin";
const rustTarget = process.env.TAURI_TARGET || "";

if (isMac) {
    if (rustTarget.includes("x86_64") || process.argv.includes("x86_64")) {
        commandPrefix = "arch -x86_64 ";
        targetArchFlag = " --target-arch x86_64";
    } else if (
        rustTarget.includes("aarch64") ||
        process.argv.includes("aarch64")
    ) {
        targetArchFlag = " --target-arch arm64";
    }
}

execSync(
    `${commandPrefix}pyinstaller --clean -y --onedir --noconsole --name fstdict-server ` +
        `--add-data "${addDataStatic}" --add-data "${addDataConfig}" --add-data "${addDataCgeventConfig}" --add-data "${addDataFfmpeg}"${targetArchFlag} ` +
        `fstdict-server.py`,
    { cwd: pythonDir, stdio: "inherit" },
);

// 6. Clean old sidecar in Tauri
if (existsSync(sidecarTargetDir)) {
    rmSync(sidecarTargetDir, { recursive: true });
}

// 7. Copy PyInstaller output to sidecars/
cpSync(pyDistDir, sidecarTargetDir, { recursive: true });
console.log(`  ✓ Sidecar → src-tauri/sidecars/fstdict-server/`);

console.log("\n✓ Python sidecar build complete\n");
