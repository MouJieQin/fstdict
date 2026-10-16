<template>
    <div class="setting-container">
        <el-scrollbar>
            <el-icon size="20" class="sticky-session-config-icon" @click="emit('toggle:config-panel')">
                <Expand />
            </el-icon>
            <p class="system-config-title">{{ $t('sessionSettings.title') }}</p>
            <el-form v-if="localSystemConfig" :model="localSystemConfig" label-width="auto" class="config-form">
                <div class="config-class">
                    <p class="config-class-title">Avatar</p>
                    <!-- <el-avatar v-if="localSystemConfig.appearance.user_avatar_url" :src="userAvatarUrl" fit="cover"
                        size="large" @click="showUserPreview = true" style="cursor: pointer; margin-bottom: 20px;" />
                    <el-image-viewer v-if="showUserPreview" :url-list="[userAvatarUrl]" show-progress :initial-index="0"
                        @close="showUserPreview = false" /> -->
                    <div>
                        <el-upload class="avatar-uploader" drag action="http://localhost:4999/api/upload/user_avatar"
                            :show-file-list="false">
                            <el-icon class="el-icon--upload"><upload-filled /></el-icon>
                            <div class="el-upload__text">
                                Drop file here or <em>click to upload</em>
                            </div>
                            <template #tip>
                                <div class="el-upload__tip">
                                    jpg/png files with a size less than 10MB
                                </div>
                            </template>
                        </el-upload>
                    </div>
                </div>

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
        </el-scrollbar>
    </div>
</template>

<script setup lang="ts">
import { reactive, ref, watch, computed, onBeforeMount, Text } from 'vue'

import type { PropType } from 'vue'
import { useI18n } from 'vue-i18n'


import { SessionWebSocketService } from '@/common/session-websocket-client'

// icon
import { Expand, UploadFilled } from '@element-plus/icons-vue'



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

const emit = defineEmits<{
    (e: 'toggle:config-panel'): void
}>()

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

<style scoped>
/* 外层容器兜底 */
:deep(.el-upload) {
    background: transparent !important;
}

:deep(.el-upload:hover) {
    background: transparent !important;
}

/* 重点：拖拽上传盒子 el-upload-dragger */
:deep(.el-upload-dragger) {
    background-color: transparent !important;
    border: 1px dashed var(--el-border-color);
}

/* 拖拽悬浮 dragover 状态，也要清空背景（否则拖文件上去会出现一块浅色底色） */
:deep(.el-upload-dragger.is-dragover) {
    background-color: transparent !important;
    border: 1px dashed var(--el-color-primary);
}

/* picture-card 卡片模式 */
:deep(.el-upload--picture-card) {
    background-color: transparent !important;
}

/* 文件列表每一项背景透明 */
:deep(.el-upload-list__item) {
    background-color: transparent !important;
}

/* 文件项hover背景也清掉，液态玻璃界面很关键 */
:deep(.el-upload-list__item:hover) {
    background-color: transparent !important;
}
</style>