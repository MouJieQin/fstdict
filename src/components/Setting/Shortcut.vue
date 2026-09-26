<template>
    <div class="setting-container">
        <el-form v-if="localSystemConfig" :model="localSystemConfig" label-width="auto" class="config-form">
            <!-- Shortcuts Section -->
            <div class="config-class">
                <p class="config-class-title">{{ t('settings.shortcut') }}</p>

                <el-form-item :label="t('settings.toggleHelper')">
                    <HotkeyInput v-model="localSystemConfig.shortcuts.toggle_selection"
                        @update:modelValue="updateToggleSelectShortcuts" />
                </el-form-item>

                <el-form-item :label="t('settings.screenshotOcr')">
                    <HotkeyInput v-model="localSystemConfig.shortcuts.screenshot_ocr"
                        @update:modelValue="updateScreenshotOcrShortcuts" />
                </el-form-item>
            </div>
        </el-form>
    </div>
</template>

<script setup lang="ts">
import { reactive, ref, watch, computed, onBeforeMount, Text } from 'vue'
import HotkeyInput from '@/components/HotkeyInput.vue'
import type { PropType } from 'vue'
import { useI18n } from 'vue-i18n'

// Stores & utilities
import { useSystemConfigStore } from '@/stores'
import { invoke } from '@tauri-apps/api/core'
import { safeDeepClone } from '@/common/utility'

// Types & constants
import { SessionWebSocketService } from '@/common/session-websocket-client'
import {
    TAURI_CMD,
} from '@/common/constants'

// ─── Props & Emits ────────────────────────────────────────────────
const props = defineProps({
    webSocket: {
        type: [Object, null] as PropType<SessionWebSocketService | null>,
        required: true,
    }
})

const { t } = useI18n()

// ─── Stores ──────────────────────────────────────────────────────
const systemConfigStore = useSystemConfigStore()

// ─── Local State ─────────────────────────────────────────────────
const localSystemConfig = ref<any>(null)


// ─── Handle update shortcuts ──────────────────────────────────────────────
const updateToggleSelectShortcuts = (shortcuts: string[]) => {
    persistShortcutConfig('toggle_selection', shortcuts)
}

const updateScreenshotOcrShortcuts = (shortcuts: string[]) => {
    persistShortcutConfig('screenshot_ocr', shortcuts)
}


// ─── Persistence Helpers ────────────────────────────────────────
function persistSystemConfig(): void {
    props.webSocket?.sendUpdateSystemConfig(localSystemConfig.value)
}

function persistShortcutConfig(shortcutName: string, shortcuts: string[]): void {
    props.webSocket?.sendUpdateShortcutSystemConfig(shortcutName, shortcuts)
}


// ─── Show Updater Window ──────────────────────────────────────────────
async function openUpdater(): Promise<void> {
    try {
        await invoke(TAURI_CMD.SHOW_UPDATER_WINDOW)
    } catch (error) {
        console.error('Failed to show updater window:', error)
    }
}

// ─── Watchers ───────────────────────────────────────────────────
watch(
    () => systemConfigStore.systemConfig,
    async (value) => {
        localSystemConfig.value = safeDeepClone(value)
    },
    { deep: true, immediate: true }
)

</script>
