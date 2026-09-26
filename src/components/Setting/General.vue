<template>
    <div class="setting-container">
        <el-form v-if="localSystemConfig" :model="localSystemConfig" label-width="auto" class="config-form">
            <!-- Appearance Section -->
            <div class="config-class">
                <p class="config-class-title">{{ $t('settings.appearance') }}</p>
                <el-form-item :label="$t('settings.update')">
                    <el-button @click="openUpdater">
                        {{ $t('settings.checkUpdate') }}
                    </el-button>
                </el-form-item>
                <el-form-item :label="$t('settings.theme')">
                    <el-radio-group v-model="appTheme">
                        <el-radio-button value="light">
                            <div class="config-radio-button">
                                <el-icon class="config-radio-icon">
                                    <Sunny />
                                </el-icon>
                                <span>{{ $t('settings.light') }}</span>
                            </div>
                        </el-radio-button>
                        <el-radio-button value="dark">
                            <div class="config-radio-button">
                                <el-icon class="config-radio-icon">
                                    <Moon />
                                </el-icon>
                                <span>{{ $t('settings.dark') }}</span>
                            </div>
                        </el-radio-button>
                        <el-radio-button value="auto">
                            <div class="config-radio-button">
                                <el-icon class="config-radio-icon">
                                    <SwitchFilled />
                                </el-icon>
                                <span>{{ $t('settings.system') }}</span>
                            </div>
                        </el-radio-button>
                    </el-radio-group>
                </el-form-item>

                <el-form-item :label="$t('settings.language')">
                    <el-radio-group v-model="appLanguage">
                        <el-radio-button value="en">
                            <div class="config-radio-button">
                                <span>{{ $t('common.english') }}</span>
                            </div>
                        </el-radio-button>
                        <el-radio-button value="zh">
                            <div class="config-radio-button">
                                <span>{{ $t('common.chinese') }}</span>
                            </div>
                        </el-radio-button>
                        <el-radio-button value="ja">
                            <div class="config-radio-button">
                                <span>{{ $t('common.japanese') }}</span>
                            </div>
                        </el-radio-button>
                        <el-radio-button value="ko">
                            <div class="config-radio-button">
                                <span>{{ $t('common.korean') }}</span>
                            </div>
                        </el-radio-button>
                    </el-radio-group>
                </el-form-item>
            </div>
        </el-form>
    </div>
</template>

<script setup lang="ts">
import { reactive, ref, watch, computed, onBeforeMount, Text } from 'vue'
import type { PropType } from 'vue'
import { useI18n } from 'vue-i18n'

// Icons
import {
    Sunny,
    Moon,
    SwitchFilled,
} from '@element-plus/icons-vue'

// Stores & utilities
import { useSystemConfigStore } from '@/stores'
import { invoke } from '@tauri-apps/api/core'
import { safeDeepClone } from '@/common/utility'
import { setAppLocale } from '@/i18n'

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

const appTheme = computed({
    get: () => localSystemConfig.value?.appearance?.theme || 'light',
    set: (value: string) => {
        if (localSystemConfig.value) {
            localSystemConfig.value.appearance.theme = value
            persistSystemConfig()
        }
    },
})

const appLanguage = computed({
    get: () => localSystemConfig.value?.appearance?.language || 'en',
    set: (value: 'en' | 'zh') => {
        if (localSystemConfig.value) {
            localSystemConfig.value.appearance.language = value
            setAppLocale(value)
            persistSystemConfig()
        }
    },
})


// ─── Persistence Helpers ────────────────────────────────────────
function persistSystemConfig(): void {
    props.webSocket?.sendUpdateSystemConfig(localSystemConfig.value)
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
