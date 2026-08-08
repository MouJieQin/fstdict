import { execSync } from "node:child_process";
import {
    cpSync,
    rmSync,
    existsSync,
    mkdirSync,
    renameSync,
    symlinkSync,
} from "node:fs";
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

// 如果当前在 MacOS 且检测到显式指定的跨平台 Target 编译参数，强制纠正目标架构
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
            // 如果你的 windows 构建仍在使用 tar 打包 zip，可以用 tar -xzf 替换 powershell
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

// ✨ 优化：增加 --exclude-module _tkinter 清除 _tcl_data 和 _tk_data
execSync(
    `${commandPrefix}pyinstaller --clean -y --onedir --noconsole --name fstdict-server ` +
        `--exclude-module _tkinter ` +
        `--add-data "${addDataStatic}" --add-data "${addDataConfig}" --add-data "${addDataCgeventConfig}" --add-data "${addDataFfmpeg}"${targetArchFlag} ` +
        `fstdict-server.py`,
    { cwd: pythonDir, stdio: "inherit" },
);

// ✨ 7. macOS 专属优化：清除 GitHub Actions 重复打包产生的硬拷贝二进制文件，还原为相对软链接 (Symlinks)
if (isMac) {
    console.log("  Optimizing macOS PyInstaller size (Restoring Symlinks)...");
    const internalDir = join(pyDistDir, "_internal");
    const realPythonBinary = join(
        internalDir,
        "Python.framework",
        "Versions",
        "3.11",
        "Python",
    );

    if (existsSync(realPythonBinary)) {
        const rootPython = join(internalDir, "Python");
        const frameworkPython = join(internalDir, "Python.framework", "Python");
        const currentVersionPython = join(
            internalDir,
            "Python.framework",
            "Versions",
            "Current",
        );

        // 删除冗余实体副本，写入标准的 UNIX 相对路径软链结构
        if (existsSync(rootPython)) {
            rmSync(rootPython, { force: true });
            symlinkSync("./Python.framework/Versions/3.11/Python", rootPython);
        }
        if (existsSync(frameworkPython)) {
            rmSync(frameworkPython, { force: true });
            symlinkSync("./Versions/3.11/Python", frameworkPython);
        }
        if (existsSync(currentVersionPython)) {
            rmSync(currentVersionPython, { force: true });
            symlinkSync("./3.11", currentVersionPython);
        }
        console.log(
            "  ✓ Successfully replaced binary duplicates with relative symlinks.",
        );
    }
}

// 8. Clean old sidecar in Tauri
if (existsSync(sidecarTargetDir)) {
    rmSync(sidecarTargetDir, { recursive: true });
}

// 9. Copy PyInstaller output to sidecars/
// cpSync 在复制时，会默认完整保留并在目标目录中重新创建上述生成的 relative symlinks
cpSync(pyDistDir, sidecarTargetDir, { recursive: true });
console.log(`  ✓ Sidecar → src-tauri/sidecars/fstdict-server/`);

console.log("\n✓ Python sidecar build complete\n");
