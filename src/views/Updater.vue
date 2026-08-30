<template>
    <div class="center-icon-container">
        <img src="/icon.png" class="window-center-icon" />
    </div>
    <div class="updater-state">
        <p v-if="state === 'UPTODATE'" style="font-weight: bold;text-align: center;">{{ $t('updater.upToDate') }}</p>
        <p v-if="checkedAndAvailable" class="update-title">{{
            $t('updater.newVersionAvailable') }}</p>
        <p v-if="checkedAndAvailable" style="font-size:10px">{{ $t('updater.updateTip', {
            version: versionAvailable,
            currentVersion: currentVersion
        }) }}</p>
        <span class="checking-update">
            <p v-if="state === 'CHECKING' || state === 'CHECK_FAILED'">{{ $t('updater.checkingForUpdates') }}</p>
            <el-button v-if="state === 'CHECK_FAILED'" @click="checkUpdate">{{ $t('common.retry') }}</el-button>
            <el-icon v-else-if="state === 'CHECKING'" class="is-loading" size="25">
                <Loading />
            </el-icon>
            <p v-if="state === 'INSTALLING'">{{ $t('updater.installing') }}</p>
            <p v-if="state === 'RELAUNCHING'">{{ $t('updater.relaunching') }}</p>
        </span>
        <p v-if="checkedAndAvailable" class="update-title">
            {{ $t('updater.releaseNotes') }}
        </p>
        <p v-if="checkedAndAvailable" class="update-notes">
            {{ updateNotes }}
        </p>
        <el-progress v-if="state === 'DOWNLOADING'" :percentage="progressPercentage" style="padding:10px 0" />
        <el-progress v-if="state === 'DOWNLOADED' || state === 'INSTALLING' || state === 'INSTALLED'" :percentage="100"
            status="success" />
        <div style="display: flex;align-items: center;justify-content: flex-end;">
            <el-button v-if="checkedAndAvailable && isWindows()" @click="downloader">{{
                $t('updater.download')
            }}</el-button>
            <el-button v-if="checkedAndAvailable && !isWindows()" @click="downloadAndInstaller">{{
                $t('updater.downloadAndInstall')
            }}</el-button>
            <el-button v-if="state === 'DOWNLOADING'" @click="canceler">{{
                $t('common.cancel')
                }}</el-button>
            <!-- On Windows the application is automatically exited when the install step is executed due to a limitation of Windows installers. -->
            <el-button v-if="state === 'DOWNLOADED' && isWindows()" @click="installer">{{
                $t('updater.installAndRelaunch')
                }}</el-button>
            <el-button v-if="state === 'INSTALLED' && !isWindows()" @click="relauncher">{{
                $t('updater.relaunch')
                }}</el-button>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted, onBeforeUnmount, shallowRef, watch } from 'vue'
import { check } from '@tauri-apps/plugin-updater';
import type { Update } from '@tauri-apps/plugin-updater';
import { Loading } from '@element-plus/icons-vue'
import { relaunch } from '@tauri-apps/plugin-process';
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { TAURI_CMD } from '@/common/constants'
import type { UpdaterState } from '@/common/type-interface'

const { t } = useI18n()
const progressPercentage = computed(() =>
    contentLength.value ? Math.floor(downloaded.value * 100 / contentLength.value) : 0
)

const downloaded = ref<number>(0)
const contentLength = ref<number | undefined>(100)
const state = ref<UpdaterState>('CHECKING')
// ✅ shallowRef: no deep proxying, Update instance stays untouched
const update = shallowRef<Update | null>(null)

// Derived values work normally
const updateAvailable = computed(() => !!update.value)
const versionAvailable = computed(() => update.value?.version ?? '')
const currentVersion = computed(() => update.value?.currentVersion ?? '')
const updateNotes = computed(() => update.value?.body ?? '')
const checkedAndAvailable = computed(() => state.value === 'AVAILABLE')

function isWindows(): boolean {
    return typeof window !== 'undefined' && /Windows/.test(navigator.userAgent)
}

watch(() => state.value, async (value) => {
    switch (value) {
        case 'CHECKING':
            await invoke(TAURI_CMD.SET_UPDATER_WINDOW_SIZE, { width: 360.0, height: 180 })
            break;
        case 'AVAILABLE':
            if (updateAvailable) {
                await invoke(TAURI_CMD.SET_UPDATER_WINDOW_SIZE, { width: 360.0, height: 360 })
            }
            break;
        default:
            await invoke(TAURI_CMD.SET_UPDATER_WINDOW_SIZE, { width: 360.0, height: 180 })
            break;

    }
})

const checkUpdate = async () => {
    try {
        state.value = 'CHECKING'
        update.value = await check({ timeout: 30000 });
    } catch (error) {
        state.value = 'CHECK_FAILED'
        console.error(
            `Checking update error: ${error}`
        );
        return
    }
    if (!update.value) {
        state.value = 'UPTODATE'
    } else {
        state.value = 'AVAILABLE'
        console.log(
            `found update ${update.value.version} from ${update.value.date} with notes ${update.value.body}`
        );
    }
}

const downloader = async () => {
    await update.value?.download((event) => {
        switch (event.event) {
            case 'Started':
                contentLength.value = event.data.contentLength;
                console.log(`started downloading ${event.data.contentLength} bytes`);
                state.value = "DOWNLOADING"
                break;
            case 'Progress':
                downloaded.value += event.data.chunkLength;
                state.value = "DOWNLOADING"
                break;
            case 'Finished':
                console.log('download finished');
                state.value = "DOWNLOADED"
                break;
        }
    });
}

const canceler = async () => {
    await update.value?.close()
    state.value = 'CHECK_FAILED'
}

const installer = async () => {
    await update.value?.install()
}

const downloadAndInstaller = async () => {
    state.value = 'DOWNLOADING'
    if (update.value) {
        try {
            await downloader()
        } catch (error) {
            console.error(
                `download error: ${error}`
            );
            state.value = 'CHECK_FAILED'
            return
        }
        state.value = 'INSTALLING'
        console.log('installing update');
        await installer()
        state.value = 'INSTALLED'
        console.log('update installed');
    }
}

const relauncher = async () => {
    await relaunch()
}

onMounted(async () => {
    console.log("checkUpdate")
    await checkUpdate()
})

onBeforeUnmount(async () => {
    console.log("onBeforeUnmount")
    if (update.value) {
        console.log("update.value.close")
        await update.value.close()
    }
})
</script>