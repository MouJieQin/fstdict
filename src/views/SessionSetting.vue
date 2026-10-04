<template>
    <div class="setting-container">
        <p class="system-config-title">{{ $t('sessionSettings.title') }}</p>
        <el-form v-if="localSystemConfig" :model="localSystemConfig" label-width="auto" class="config-form">
            <div class="config-class">
                <!-- <p class="config-class-title">{{ $t('settings.appearance') }}</p> -->

                <el-form-item :label="$t('sessionSettings.name')">
                    <el-text truncated class="word-text">
                        {{ localSessionConfig.name }}
                    </el-text>
                </el-form-item>

                <el-form-item :label="$t('sessionSettings.dictGroup')">
                    <el-select v-model="localSessionConfig.dict_setting_option_name" filterable
                        :placeholder="$t('sessionSettings.dictGroup')" style="max-width: 240px"
                        @change="persistSessionConfig">
                        <el-option v-for="(_, name) in dictSetOptions" :key="name" :label="name" :value="name" />
                    </el-select>
                </el-form-item>

                <el-form-item :label="$t('sessionSettings.defaultFolder')">
                    <el-select v-if="localFolderConfig" v-model="localSessionConfig.default_folder.id" filterable
                        :placeholder="$t('sessionSettings.defaultFolder')" style="max-width: 240px"
                        @change="persistSessionConfig">
                        <el-option v-for="folder in folderOptions" :key="folder.id" :label="folder.name"
                            :value="folder.id" />
                    </el-select>
                </el-form-item>

                <!-- OCR Section -->
                <el-form-item :label="$t('sessionSettings.ocrLang')">
                    <el-select v-model="localSessionConfig.ocr_lang_type" filterable
                        :placeholder="$t('sessionSettings.ocrLang')" style="max-width: 240px"
                        @change="persistSessionConfig">
                        <el-option v-for="lang in ocrLanguageOptions" :key="lang" :label="lang" :value="lang" />
                    </el-select>
                </el-form-item>
            </div>
        </el-form>
    </div>
</template>

<script setup lang="ts">
import { reactive, ref, watch, computed, onBeforeMount, Text } from 'vue'

import type { PropType } from 'vue'
import { useI18n } from 'vue-i18n'


import { SessionWebSocketService } from '@/common/session-websocket-client'


// Stores & utilities
import { useFolderConfigStore, useSystemConfigStore, useDictConfigStore } from '@/stores'
import { safeDeepClone } from '@/common/utility'

// Types & constants
import type {
    SessionConfig,
    FolderConfig,
    FolderInfo,
    FolderWords,
} from '@/common/type-interface'

const { t } = useI18n()

// ─── Stores ──────────────────────────────────────────────────────
const folderConfigStore = useFolderConfigStore()
const systemConfigStore = useSystemConfigStore()

// ─── Local State ─────────────────────────────────────────────────
const localFolderConfig = ref<FolderConfig | null>(null)
const localSessionConfig = ref<SessionConfig>({} as SessionConfig)
const localSystemConfig = ref<any>(null)

// ─── Props & Emits ────────────────────────────────────────────────
const props = defineProps({
    webSocket: {
        type: [Object, null] as PropType<SessionWebSocketService | null>,
        required: true,
    },
    sessionConfig: {
        type: Object as PropType<SessionConfig>,
        required: true,
    }
})

// ─── Computed ────────────────────────────────────────────────────
const dictSetOptions = computed(() => {
    return useDictConfigStore().dictConfig?.dict_set_options || {}
})


const folderOptions = computed(() =>
    localFolderConfig.value?.folders.folder_info.map((f) => ({
        id: f.id,
        name: f.name,
    })) || []
)

const ocrLanguageOptions = computed(() =>
    Object.keys(localSystemConfig.value?.ocr?.lang_types || {})
)

// ─── Watchers ───────────────────────────────────────────────────
watch(
    () => folderConfigStore.folderConfig,
    (value) => {
        localFolderConfig.value = safeDeepClone(value)
    },
    { deep: true, immediate: true }
)

watch(
    () => props.sessionConfig,
    (value) => {
        localSessionConfig.value = safeDeepClone(value)
    },
    { deep: true, immediate: true }
)

watch(
    () => systemConfigStore.systemConfig,
    async (value) => {
        localSystemConfig.value = safeDeepClone(value)

    },
    { deep: true, immediate: true }
)

// ─── Persistence Helpers ────────────────────────────────────────
function persistSystemConfig(): void {
    props.webSocket?.sendUpdateSystemConfig(localSystemConfig.value)
}

function persistSessionConfig(): void {
    props.webSocket?.sendSessionConfig(localSessionConfig.value)
}

</script>