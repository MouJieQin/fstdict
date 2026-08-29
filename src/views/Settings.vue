<template>
    <div class="setting-container">
        <p class="system-config-title">{{ $t('settings.title') }}</p>

        <el-form v-if="localSystemConfig" :model="localSystemConfig" label-width="150px" class="config-form">
            <!-- Appearance Section -->
            <div class="config-class">
                <p class="config-class-title">{{ $t('settings.appearance') }}</p>
                <el-form-item :label="$t('settings.update')">
                    <el-button @click="openUpdater">
                        {{ $t('settings.checkUpdate') }}
                    </el-button>
                </el-form-item>
                <el-form-item :label="$t('settings.theme')">
                    <el-radio-group v-model="appTheme" size="large" fill="#6cf">
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
                    <el-radio-group v-model="appLanguage" size="large" fill="#6cf">
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

            <!-- Favorite Folders Section -->
            <div class="config-class">
                <p class="config-class-title">{{ $t('settings.favoriteFolders') }}</p>

                <el-table v-if="localFolderConfig" :data="localFolderConfig.folders.folder_info" height="350"
                    style="width: 100%" @selection-change="handleSelectionChange" stripe>
                    <el-table-column type="selection" width="55" />
                    <el-table-column fixed prop="name" :label="$t('settings.name')" width="130" show-overflow-tooltip
                        sortable />
                    <el-table-column prop="words_count" :label="$t('settings.words')" sortable />
                    <el-table-column prop="created_at" :label="$t('settings.created')" width="110" show-overflow-tooltip
                        sortable />
                    <el-table-column prop="description" :label="$t('settings.description')" width="180"
                        show-overflow-tooltip />
                    <el-table-column fixed="right" :label="$t('settings.actions')" width="160">
                        <template #default="{ row }">
                            <el-button-group>
                                <el-button :icon="Edit" size="small" @click="openEditFolder(row)"
                                    :aria-label="$t('settings.editFolderAction')" />
                                <el-button :icon="Document" size="small" @click="openFolderWords(row)"
                                    :aria-label="$t('settings.viewWords')" />
                                <el-popconfirm :confirm-button-text="$t('common.delete')" confirm-button-type="danger"
                                    :cancel-button-text="$t('common.cancel')" :icon="Delete" icon-color="#FF4949"
                                    :title="$t('settings.deleteFolder')" @confirm="deleteFolder(row.id)">
                                    <template #reference>
                                        <el-button :icon="Delete" size="small" type="danger"
                                            :aria-label="$t('settings.deleteFolder')" />
                                    </template>
                                </el-popconfirm>
                            </el-button-group>
                        </template>
                    </el-table-column>
                </el-table>

                <div class="folder-toolbar">
                    <el-button type="primary" :icon="Plus" @click="openCreateFolder">
                        {{ $t('settings.newFolder') }}
                    </el-button>
                    <el-button type="danger" :icon="Delete" @click="deleteSelectedFolders"
                        :disabled="selectedFolders.length === 0">
                        {{ $t('settings.deleteSelected') }}
                    </el-button>
                    <el-button @click="exportToAnki" :disabled="selectedFolders.length === 0">
                        <AnkiIcon :size="24" style="margin-right: 8px" />
                        {{ $t('settings.exportToAnki') }}
                    </el-button>

                    <el-select v-if="localFolderConfig" v-model="localSessionConfig.default_folder.id" filterable
                        :placeholder="$t('settings.defaultFolder')" style="margin-left: 20px; max-width: 240px"
                        @change="persistSessionConfig">
                        <el-option v-for="folder in folderOptions" :key="folder.id" :label="folder.name"
                            :value="folder.id" />
                    </el-select>
                </div>
            </div>

            <!-- Shortcuts Section -->
            <div class="config-class">
                <p class="config-class-title">{{ t('settings.shortcuts') }}</p>

                <el-form-item :label="t('settings.toggleHelper')">
                    <HotkeyInput v-model="localSystemConfig.shortcuts.toggle_selection"
                        @update:modelValue="updateToggleSelectShortcuts" />
                </el-form-item>

                <el-form-item :label="t('settings.screenshotOcr')">
                    <HotkeyInput v-model="localSystemConfig.shortcuts.screenshot_ocr"
                        @update:modelValue="updateScreenshotOcrShortcuts" />
                </el-form-item>
            </div>


            <!-- OCR Section -->
            <div class="config-class">
                <p class="config-class-title">{{ $t('settings.ocr') }}</p>
                <el-select v-model="localSessionConfig.ocr_lang_type" filterable
                    :placeholder="$t('settings.defaultOcrLang')" style="max-width: 240px"
                    @change="persistSessionConfig">
                    <el-option v-for="lang in ocrLanguageOptions" :key="lang" :label="lang" :value="lang" />
                </el-select>
            </div>

            <!-- Helper Section -->
            <div class="config-class">
                <p class="config-class-title">{{ $t('settings.helper') }}</p>
                <p class="config-class-desc">{{ $t('settings.helperPermission') }}</p>
                <el-switch v-model="helperEnabled" @change="handleHelperToggle" />
            </div>
        </el-form>

        <!-- Create / Edit Folder Dialog -->
        <el-dialog v-model="folderDialogVisible" :title="folderDialogTitle" width="500" align-center
            @keydown.enter.prevent.stop>
            <el-form ref="folderFormRef" :model="folderForm" :rules="folderFormRules" label-width="100px">
                <el-form-item :label="$t('settings.name')" prop="name">
                    <el-input v-model="folderForm.name" autocomplete="off" />
                </el-form-item>
                <el-form-item :label="$t('settings.description')" prop="description">
                    <el-input v-model="folderForm.description" autocomplete="off" type="textarea" />
                </el-form-item>
            </el-form>
            <template #footer>
                <div class="dialog-footer">
                    <el-button @click="folderDialogVisible = false">{{ $t('common.cancel') }}</el-button>
                    <el-button type="primary" @click="submitFolderForm">
                        {{ $t('common.save') }}
                    </el-button>
                </div>
            </template>
        </el-dialog>

        <!-- Anki Export Progress Dialog -->
        <el-dialog v-model="ankiDialogVisible" :title="ankiExportTitle" width="700" :close-on-click-modal="false"
            :close-on-press-escape="false" draggable :show-close="false">
            <div v-for="folder in selectedFolders" :key="folder.id" class="config-class">
                <p class="config-class-title">{{ folder.name }}</p>
                <AnkiProgress :web-socket="webSocket" :anki-progress="ankiProgresses[folder.name] || {}"
                    :anki-dialog-visible="ankiDialogVisible" />
            </div>
            <template #footer>
                <div class="dialog-footer">
                    <el-button v-if="!allAnkiCompleted" type="danger" @click="cancelAnkiExport">
                        {{ $t('anki.cancelExport') }}
                    </el-button>
                    <el-button v-else type="primary" @click="ankiDialogVisible = false">
                        {{ $t('common.close') }}
                    </el-button>
                </div>
            </template>
        </el-dialog>

        <!-- Favorite Words Dialog -->
        <el-dialog v-model="favoriteWordsVisible" fullscreen>
            <FavoriteWords :favorite-words-dialog-visible="favoriteWordsVisible" :web-socket="webSocket"
                @update-visible="favoriteWordsVisible = $event" :favorite-words="viewingFolderWords"
                :folder-name="viewingFolderName" :folder-id="viewingFolderId" />
        </el-dialog>
    </div>
</template>

<script setup lang="ts">
import { reactive, ref, watch, computed, onBeforeMount } from 'vue'
import type { PropType, FormInstance, FormRules } from 'vue'
import { ElMessageBox } from 'element-plus'
import { useI18n } from 'vue-i18n'

// Icons
import {
    Edit,
    Delete,
    Document,
    Plus,
    Sunny,
    Moon,
    SwitchFilled,
} from '@element-plus/icons-vue'

// Components
import AnkiIcon from '@/components/Icons/AnkiIcon.vue'
import FavoriteWords from '@/components/Dialogs/FavoriteWords.vue'
import AnkiProgress from '@/components/Dialogs/AnkiProgress.vue'

// Stores & utilities
import { useFolderConfigStore, useSystemConfigStore } from '@/stores'
import { isMacOS, checkAccessibilitySafe, requestAccessibilitySafe } from '@/common/accessibility'
import { invoke } from '@tauri-apps/api/core'
import { safeDeepClone } from '@/common/utility'
import HotkeyInput from '@/components/HotkeyInput.vue'
import { initPlatformDetection } from '@/common/hotkey'
import { setAppLocale } from '@/i18n'

// Types & constants
import type {
    SessionConfig,
    FolderConfig,
    FolderInfo,
    FolderWords,
} from '@/common/type-interface'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import {
    MAX_FOLDER_NAME_LENGTH,
    TAURI_CMD,
    ANKI_STATE,
} from '@/common/constants'

// ─── Props & Emits ────────────────────────────────────────────────
const props = defineProps({
    settingDialogVisible: {
        type: Boolean,
        required: true,
    },
    webSocket: {
        type: [Object, null] as PropType<SessionWebSocketService | null>,
        required: true,
    },
    sessionConfig: {
        type: Object as PropType<SessionConfig>,
        required: true,
    },
    folderWords: {
        type: Object as PropType<FolderWords>,
        required: true,
        default: () => ({}),
    },
    ankiProgress: {
        type: Object,
        required: true,
        default: () => ({}),
    },
})

const emit = defineEmits<{
    (e: 'update-visible', visible: boolean): void
}>()

const { t } = useI18n()

// ─── Stores ──────────────────────────────────────────────────────
const folderConfigStore = useFolderConfigStore()
const systemConfigStore = useSystemConfigStore()

// ─── Local State ─────────────────────────────────────────────────
const localFolderConfig = ref<FolderConfig | null>(null)
const localSessionConfig = ref<SessionConfig>({} as SessionConfig)
const localSystemConfig = ref<any>(null)
const ankiProgresses = ref<Record<string, any>>({})

const selectedFolders = ref<FolderInfo[]>([])
const helperEnabled = ref(false)

// Folder dialog state
const folderDialogVisible = ref(false)
const isCreatingFolder = ref(true)
const editingFolderId = ref<number | null>(null)
const folderFormRef = ref<FormInstance>()

const folderForm = reactive({
    name: '',
    description: '',
})

const folderFormRules = reactive<FormRules<typeof folderForm>>({
    name: [{ validator: validateFolderName, trigger: 'blur' }],
    description: [{ validator: () => true, trigger: 'blur' }],
})

// Dialog visibility
const ankiDialogVisible = ref(false)
const favoriteWordsVisible = ref(false)
const viewingFolderId = ref(0)
const viewingFolderName = ref('')

// ─── Computed ────────────────────────────────────────────────────
const folderDialogTitle = computed(() =>
    isCreatingFolder.value ? t('settings.createFolder') : t('settings.editFolder')
)

const ankiExportTitle = computed(() =>
    t('anki.exportTitle', { count: selectedFolders.value.length })
)

const folderOptions = computed(() =>
    localFolderConfig.value?.folders.folder_info.map((f) => ({
        id: f.id,
        name: f.name,
    })) || []
)

const ocrLanguageOptions = computed(() =>
    Object.keys(localSystemConfig.value?.ocr?.lang_types || {})
)

const allAnkiCompleted = computed(() => {
    for (const folder of selectedFolders.value) {
        const progress = ankiProgresses.value[folder.name]
        if (!progress) return false
        const finishedStates = [ANKI_STATE.DONE, ANKI_STATE.CANCELED, ANKI_STATE.ERROR]
        if (!finishedStates.includes(progress.type)) {
            return false
        }
    }
    return true
})

const viewingFolderWords = computed(() =>
    props.folderWords[viewingFolderId.value] || []
)

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

// ─── Validators ──────────────────────────────────────────────────
function validateFolderName(
    _rule: unknown,
    value: string,
    callback: (error?: Error) => void
): void {
    const trimmed = value?.trim()

    if (!trimmed) {
        callback(new Error(t('settings.folderNameRequired')))
        return
    }

    if (trimmed.length > MAX_FOLDER_NAME_LENGTH) {
        callback(new Error(t('settings.folderNameTooLong', { max: MAX_FOLDER_NAME_LENGTH })))
        return
    }

    const duplicate = localFolderConfig.value?.folders.folder_info.some(
        (f) => f.name === trimmed && f.id !== editingFolderId.value
    )

    if (duplicate && isCreatingFolder.value) {
        callback(new Error(t('settings.folderNameExists')))
        return
    }

    callback()
}

// ─── Persistence Helpers ────────────────────────────────────────
function persistSystemConfig(): void {
    props.webSocket?.sendUpdateSystemConfig(localSystemConfig.value)
}

function persistSessionConfig(): void {
    props.webSocket?.sendSessionConfig(localSessionConfig.value)
}

// ─── Folder Actions ─────────────────────────────────────────────
function openCreateFolder(): void {
    isCreatingFolder.value = true
    editingFolderId.value = null
    folderForm.name = ''
    folderForm.description = ''
    folderDialogVisible.value = true
}

function openEditFolder(row: FolderInfo): void {
    isCreatingFolder.value = false
    editingFolderId.value = row.id
    folderForm.name = row.name
    folderForm.description = row.description
    folderDialogVisible.value = true
}

function submitFolderForm(): void {
    if (!folderFormRef.value) return

    folderFormRef.value.validate((valid) => {
        if (!valid) return

        const name = folderForm.name.trim()
        const description = folderForm.description.trim()

        if (isCreatingFolder.value) {
            props.webSocket?.sendCreateFolder(name, description)
        } else if (editingFolderId.value !== null) {
            props.webSocket?.sendUpdateFolder(editingFolderId.value, name, description)
        }

        folderDialogVisible.value = false
    })
}

function deleteFolder(id: number): void {
    props.webSocket?.sendDeleteFolder(id)
}

async function deleteSelectedFolders(): Promise<void> {
    const count = selectedFolders.value.length
    if (count === 0) return

    try {
        await ElMessageBox.confirm(
            t('settings.deleteConfirmMsg', { count }),
            t('settings.deleteConfirmTitle'),
            {
                confirmButtonText: t('settings.deleteButton'),
                cancelButtonText: t('common.cancel'),
                type: 'warning',
                confirmButtonType: 'danger',
                appendTo: '.setting-container',
            }
        )

        for (const folder of selectedFolders.value) {
            props.webSocket?.sendDeleteFolder(folder.id)
        }
        selectedFolders.value = []
    } catch {
        // User cancelled
    }
}

function openFolderWords(row: FolderInfo): void {
    viewingFolderId.value = row.id
    viewingFolderName.value = row.name
    props.webSocket?.sendFavoriteWordsRequest(row.id)
    favoriteWordsVisible.value = true
}

function handleSelectionChange(selection: FolderInfo[]): void {
    selectedFolders.value = selection
}

// ─── Anki Export ────────────────────────────────────────────────
function exportToAnki(): void {
    ankiProgresses.value = {}
    ankiDialogVisible.value = true

    for (const folder of selectedFolders.value) {
        props.webSocket?.sendUpdateToAnki(folder.name, folder.id)
    }
}

async function cancelAnkiExport(): Promise<void> {
    if (allAnkiCompleted.value) {
        ankiDialogVisible.value = false
        return
    }

    try {
        await ElMessageBox.confirm(
            t('anki.cancelConfirm'),
            t('anki.cancelExport'),
            {
                confirmButtonText: t('anki.cancelButton'),
                cancelButtonText: t('anki.continueButton'),
                type: 'warning',
                appendTo: '.anki-dialog',
            }
        )
        props.webSocket?.sendCancelAnkiUpdate()
    } catch {
        // User chose to continue
    }
}

// ─── Helper Toggle ──────────────────────────────────────────────
async function handleHelperToggle(enabled: boolean): Promise<void> {
    if (!enabled) return

    if (isMacOS()) {
        const hasAccess = await checkAccessibilitySafe()
        if (!hasAccess) {
            await requestAccessibilitySafe()
            helperEnabled.value = false
            return
        }

        try {
            await invoke(TAURI_CMD.LAUNCH_CGEVENT_SERVER)
            await invoke(TAURI_CMD.LAUNCH_HELPER)
        } catch (error) {
            console.error('Failed to launch helper sidecar:', error)
        }
    }
}

// ─── Handle update shortcuts ──────────────────────────────────────────────
const updateToggleSelectShortcuts = (shortcuts: string[]) => {
    localSystemConfig.value.shortcuts.toggle_selection = shortcuts
    persistSystemConfig()
}

const updateScreenshotOcrShortcuts = (shortcuts: string[]) => {
    localSystemConfig.value.shortcuts.screenshot_ocr = shortcuts
    persistSystemConfig()
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
    (value) => {
        localSystemConfig.value = safeDeepClone(value)
    },
    { deep: true, immediate: true }
)

watch(
    () => props.ankiProgress,
    (value) => {
        ankiProgresses.value = safeDeepClone(value)
    },
    { deep: true }
)

// ─── Lifecycle ──────────────────────────────────────────────────
onBeforeMount(() => {
    props.webSocket?.sendFolderConfig()
    initPlatformDetection()
})
</script>

<style scoped>
.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
}

.folder-toolbar {
    margin-top: 20px;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
}
</style>