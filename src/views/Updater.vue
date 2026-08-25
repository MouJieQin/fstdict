<template>
    <div>
        <img src="/icon.png" class="updater-icon-container" />
    </div>
    <el-progress :percentage="progressPercentage" :stroke-width="15" striped />
</template>

<script lang="ts" setup>
import { ref, computed, onMounted } from 'vue'
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

const progressPercentage = computed(() =>
    contentLength.value ? Math.floor(downloaded.value * 100 / contentLength.value) : 0
)

const downloaded = ref<number>(0)
const contentLength = ref<number | undefined>(0)
const updater = async () => {
    const update = await check();
    if (update) {
        console.log(
            `found update ${update.version} from ${update.date} with notes ${update.body}`
        );
        // alternatively we could also call update.download() and update.install() separately
        await update.downloadAndInstall((event) => {
            switch (event.event) {
                case 'Started':
                    contentLength.value = event.data.contentLength;
                    break;
                case 'Progress':
                    downloaded.value += event.data.chunkLength;
                    break;
                case 'Finished':
                    console.log('download finished');
                    break;
            }
        });

        console.log('update installed');
        await relaunch();
    }
}
onMounted(async () => {
    await updater()
})
</script>