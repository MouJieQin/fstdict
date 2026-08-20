<template>
    <div class="setting-container">
        <p class="system-config-title">Settings</p>

        <el-form v-if="localSystemConfig" :model="localSystemConfig" label-width="150px" class="config-form">
            <!-- Appearance Section -->
            <div class="config-class">
                <p class="config-class-title">Appearance</p>
                <el-form-item label="Theme">
                    <el-radio-group v-model="appTheme" size="large" fill="#6cf">
                        <el-radio-button value="light">
                            <div class="config-radio-button">
                                <el-icon class="config-radio-icon">
                                    <Sunny />
                                </el-icon>
                                <span>Light</span>
                            </div>
                        </el-radio-button>
                        <el-radio-button value="dark">
                            <div class="config-radio-button">
                                <el-icon class="config-radio-icon">
                                    <Moon />
                                </el-icon>
                                <span>Dark</span>
                            </div>
                        </el-radio-button>
                        <el-radio-button value="auto">
                            <div class="config-radio-button">
                                <el-icon class="config-radio-icon">
                                    <SwitchFilled />
                                </el-icon>
                                <span>System</span>
                            </div>
                        </el-radio-button>
                    </el-radio-group>
                </el-form-item>
            </div>

            <!-- Favorite Folders Section -->
            <div class="config-class">
                <p class="config-class-title">Favorite Folders</p>

                <el-table v-if="localFolderConfig" :data="localFolderConfig.folders.folder_info" height="350"
                    style="width: 100%" @selection-change="handleSelectionChange" stripe>
                    <el-table-column type="selection" width="55" />
                    <el-table-column fixed prop="name" label="Name" width="130" show-overflow-tooltip sortable />
                    <el-table-column prop="words_count" label="Words" sortable />
                    <el-table-column prop="created_at" label="Created" width="110" show-overflow-tooltip sortable />
                    <el-table-column prop="description" label="Description" width="180" show-overflow-tooltip />
                    <el-table-column fixed="right" label="Actions" width="160">
                        <template #default="{ row }">
                            <el-button-group>
                                <el-button :icon="Edit" size="small" @click="openEditFolder(row)"
                                    aria-label="Edit folder" />
                                <el-button :icon="Document" size="small" @click="openFolderWords(row)"
                                    aria-label="View words" />
                                <el-popconfirm confirm-button-text="Delete" confirm-button-type="danger"
                                    cancel-button-text="Cancel" :icon="Delete" icon-color="#FF4949"
                                    title="Delete this folder?" @confirm="deleteFolder(row.id)">
                                    <template #reference>
                                        <el-button :icon="Delete" size="small" type="danger"
                                            aria-label="Delete folder" />
                                    </template>
                                </el-popconfirm>
                            </el-button-group>
                        </template>
                    </el-table-column>
                </el-table>

                <div class="folder-toolbar">
                    <el-button type="primary" :icon="Plus" @click="openCreateFolder">
                        New Folder
                    </el-button>
                    <el-button type="danger" :icon="Delete" @click="deleteSelectedFolders"
                        :disabled="selectedFolders.length === 0">
                        Delete Selected
                    </el-button>
                    <el-button @click="exportToAnki" :disabled="selectedFolders.length === 0">
                        <AnkiIcon :size="24" style="margin-right: 8px" />
                        Export to Anki
                    </el-button>

                    <el-select v-if="localFolderConfig" v-model="localSessionConfig.default_folder.id" filterable
                        placeholder="Default Folder" style="margin-left: 20px; max-width: 240px"
                        @change="persistSessionConfig">
                        <el-option v-for="folder in folderOptions" :key="folder.id" :label="folder.name"
                            :value="folder.id" />
                    </el-select>
                </div>
            </div>

            <!-- OCR Section -->
            <div class="config-class">
                <p class="config-class-title">OCR</p>
                <el-select v-model="localSessionConfig.ocr_lang_type" filterable placeholder="Default OCR Language"
                    style="max-width: 240px" @change="persistSessionConfig">
                    <el-option v-for="lang in ocrLanguageOptions" :key="lang" :label="lang" :value="lang" />
                </el-select>
            </div>

            <!-- Helper Section -->
            <div class="config-class">
                <p class="config-class-title">Helper</p>
                <p class="config-class-desc">Helper accessibility permissions</p>
                <el-switch v-model="helperEnabled" @change="handleHelperToggle" />
            </div>
        </el-form>

        <!-- Create / Edit Folder Dialog -->
        <el-dialog v-model="folderDialogVisible" :title="folderDialogTitle" width="500" align-center
            @keydown.enter.prevent.stop>
            <el-form ref="folderFormRef" :model="folderForm" :rules="folderFormRules" label-width="100px">
                <el-form-item label="Name" prop="name">
                    <el-input v-model="folderForm.name" autocomplete="off" />
                </el-form-item>
                <el-form-item label="Description" prop="description">
                    <el-input v-model="folderForm.description" autocomplete="off" type="textarea" />
                </el-form-item>
            </el-form>
            <template #footer>
                <div class="dialog-footer">
                    <el-button @click="folderDialogVisible = false">Cancel</el-button>
                    <el-button type="primary" @click="submitFolderForm">
                        Save
                    </el-button>
                </div>
            </template>
        </el-dialog>

        <!-- Anki Export Progress Dialog -->
        <el-dialog v-model="ankiDialogVisible" :title="`Exporting ${selectedFolders.length} folder(s) to Anki`"
            width="700" :close-on-click-modal="false" :close-on-press-escape="false" draggable :show-close="false">
            <div v-for="folder in selectedFolders" :key="folder.id" class="config-class">
                <p class="config-class-title">{{ folder.name }}</p>
                <AnkiProgress :web-socket="webSocket" :anki-progress="ankiProgresses[folder.name] || {}"
                    :anki-dialog-visible="ankiDialogVisible" />
            </div>
            <template #footer>
                <div class="dialog-footer">
                    <el-button v-if="!allAnkiCompleted" type="danger" @click="cancelAnkiExport">
                        Cancel
                    </el-button>
                    <el-button v-else type="primary" @click="ankiDialogVisible = false">
                        Close
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
    isCreatingFolder.value ? 'Create Folder' : 'Edit Folder'
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

// ─── Validators ──────────────────────────────────────────────────
function validateFolderName(
    _rule: unknown,
    value: string,
    callback: (error?: Error) => void
): void {
    const trimmed = value?.trim()

    if (!trimmed) {
        callback(new Error('Folder name is required'))
        return
    }

    if (trimmed.length > MAX_FOLDER_NAME_LENGTH) {
        callback(new Error(`Name must be ${MAX_FOLDER_NAME_LENGTH} characters or fewer`))
        return
    }

    const duplicate = localFolderConfig.value?.folders.folder_info.some(
        (f) => f.name === trimmed && f.id !== editingFolderId.value
    )

    if (duplicate && isCreatingFolder.value) {
        callback(new Error('A folder with this name already exists'))
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
            `Delete ${count} selected folder(s)? This cannot be undone.`,
            'Confirm Deletion',
            {
                confirmButtonText: 'Delete',
                cancelButtonText: 'Cancel',
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
            'Some folders are still processing. Cancel the export?',
            'Cancel Export',
            {
                confirmButtonText: 'Cancel Export',
                cancelButtonText: 'Continue',
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