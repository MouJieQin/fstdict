<template>
    <el-config-provider :locale="elementPlusLocale">
        <main id="app" class="flex-grow container mx-auto px-4 py-8">
            <div v-if="showGlassOverlay" class="app-glass-mimic-background-wrapper">
                <div class="app-glass-blur-wrapper">
                    <router-view />
                </div>
            </div>
            <div v-else>
                <router-view />
            </div>
        </main>
    </el-config-provider>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { useTheme } from '@/common/use-theme'
import { elementPlusLocale } from '@/i18n'
import { platform } from '@tauri-apps/plugin-os'
import { isTauri } from '@tauri-apps/api/core'

const showGlassOverlay = ref(false)


const initGlassOverlay = () => {
    if (!isTauri()) {
        showGlassOverlay.value = false
        return
    } else {
        if (platform() === 'macos') {
            showGlassOverlay.value = false
        } else {
            showGlassOverlay.value = true
        }
    }
}

const { initTheme, watchSystemTheme } = useTheme()

onMounted(() => {
    initGlassOverlay()
    initTheme()
    watchSystemTheme()
})
</script>