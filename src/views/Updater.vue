<template>
    <div>
        <img src="/icon.png" class="updater-icon-container" />
    </div>
    <div class="updater-state">
        <p v-if="checkedAndAvailable" class="update-title">{{
            $t('updater.NewVersionAvailable') }}</p>
        <p v-if="checkedAndAvailable" style="font-size:10px">{{ $t('updater.UpdateTip', {
            version: versionAvailable,
            currentVersion: currentVersion
        }) }}</p>
        <span class="checking-update">
            <p v-if="state === 'CHECKING' || state === 'CHECK_FAILED'">{{ $t('updater.CheckingForUpdates') }}</p>
            <el-button v-if="state === 'CHECK_FAILED'" @click="checkUpdate">{{ $t('common.retry') }}</el-button>
            <el-icon v-else-if="state === 'CHECKING'" class="is-loading" size="25">
                <Loading />
            </el-icon>
            <p v-if="state === 'INSTALLING'">{{ $t('updater.Installing') }}</p>
            <p v-if="state === 'RELAUNCHING'">{{ $t('updater.Relaunching') }}</p>
        </span>
        <p v-if="checkedAndAvailable" class="update-title">
            {{ $t('updater.ReleaseNotes') }}
        </p>
        <p v-if="checkedAndAvailable" class="update-notes">
            {{ updateNotes }}
        </p>
        <div style="display: flex;align-items: center;justify-content: flex-end;">
            <el-button v-if="checkedAndAvailable" @click="installUpdate">{{
                $t('updater.InstallUpdate')
                }}</el-button>
        </div>
        <el-progress v-if="state === 'DOWNLOADING'" :percentage="progressPercentage" :stroke-width="15" striped />
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
const checkedAndAvailable = computed(() => state.value === 'CHECKED' && updateAvailable)

watch(() => state.value, async (value) => {
    switch (value) {
        case 'CHECKING':
            await invoke(TAURI_CMD.SET_UPDATER_WINDOW_SIZE, { width: 360.0, height: 180 })
            break;
        case 'CHECKED':
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
    state.value = 'CHECKED'
    if (update.value) {
        console.log(
            `found update ${update.value.version} from ${update.value.date} with notes ${update.value.body}`
        );
    }
}

const downloadAndInstall = async () => {
    // alternatively we could also call update.download() and update.install() separately
    if (!update.value) {
        return
    }
    await update.value.downloadAndInstall((event) => {
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
                state.value = "INSTALLING"
                break;
        }
    });
}

const installUpdate = async () => {
    state.value = 'DOWNLOADING'
    if (update.value) {
        try {
            await downloadAndInstall()
        } catch (error) {
            console.error(
                `downloadAndInstall error: ${error}`
            );
            state.value = 'CHECK_FAILED'
            return
        }
        console.log('update installed');
        state.value = 'RELAUNCHING'
        await relaunch();
    }
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