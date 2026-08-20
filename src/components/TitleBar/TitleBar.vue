<template>
    <div>
        <!-- macOS-style title bar with drag region -->
        <div data-tauri-drag-region class="floating-window-titlebar">
            <div @mousedown.stop>
                <WordOptionsAutoComplete :web-socket="webSocket" :env="env" :redirect-word="redirectWord"
                    :redirect-history-word="redirectHistoryWord" :word-options="wordOptions"
                    :session-config="sessionConfig" :search-history="searchHistory"
                    @change:keyword="emit('change:keyword', $event)"
                    :show-popover-word-options="showPopoverWordOptions" />
            </div>

            <el-button-group class="floating-window-titlebar-button-container" @mousedown.stop>
                <el-button :icon="ArrowLeftBold" text @click="goBack" class="floating-window-titlebar-button"
                    size="small" :disabled="!canGoBack" id="titlebar-history-back-button" />
                <el-button :icon="ArrowRightBold" text @click="goForward" class="floating-window-titlebar-button"
                    size="small" :disabled="!canGoForward" id="titlebar-history-forward-button" />

                <el-tooltip v-if="showFavoriteTooltip" :content="$t('titleBar.setDefaultFolderFirst')"
                    placement="bottom">
                    <el-button :icon="BsHeart" text class="floating-window-titlebar-button" size="small" disabled />
                </el-tooltip>
                <el-button v-else :icon="isWordFavorited ? BsHeartFill : BsHeart" text @click="toggleFavorite"
                    class="floating-window-titlebar-button" size="small" :disabled="!canFavorite" />

                <el-button :icon="Edit" text @click="openNoteDialog" class="floating-window-titlebar-button"
                    size="small" :disabled="!lastSearchKeyword" />

                <el-button :icon="ImBooks" text id="titlebar-dictss-button"
                    @click="dictDialogVisible = !dictDialogVisible" class="floating-window-titlebar-button"
                    size="small" />
                <el-button :icon="Setting" text id="titlebar-setting-button"
                    @click="settingsDialogVisible = !settingsDialogVisible" class="floating-window-titlebar-button"
                    size="small" />

                <el-button v-if="showPinButton()" :icon="isPinned ? BsPinAngleFill : BsPin" text @click="togglePin"
                    class="floating-window-titlebar-button" size="small" />

                <el-dropdown id="titlebar-sessions-button" trigger="click" placement="bottom-end"
                    class="floating-window-titlebar-button" @command="handleSessionCommand">
                    <el-button :icon="PiUserSwitch" text size="small" style="font-size: 15px" />
                    <template #dropdown>
                        <el-dropdown-menu style="max-height: 60vh; overflow-y: auto;">
                            <el-dropdown-item v-for="session in sessionsNameId" :key="session.id"
                                :class="{ 'is-active': session.id === sessionId }"
                                :command="{ cmd: 'switch', id: session.id }">
                                <el-icon v-if="session.id === sessionId" style="color: var(--el-color-primary)" size="20">
                                    <BiUserCheck />
                                </el-icon>
                                <el-icon v-else>
                                    <BiUser />
                                </el-icon>
                                <span>{{ session.name }}</span>
                            </el-dropdown-item>
                            <!-- Session dropdown menu -->
                            <el-dropdown-item divided :command="{ cmd: 'create', id: -1 }">
                                <el-icon>
                                    <BiUserPlus style="color: var(--el-color-primary)" />
                                </el-icon>
                                <span>{{ $t('session.newSession') }}</span>
                            </el-dropdown-item>
                            <el-dropdown-item :command="{ cmd: 'rename', id: -1 }">
                                <el-icon>
                                    <LiaUserEditSolid style="color: var(--el-color-success)" />
                                </el-icon>
                                <span>{{ $t('session.renameSession') }}</span>
                            </el-dropdown-item>
                            <el-dropdown-item :command="{ cmd: 'remove', id: -1 }">
                                <el-icon>
                                    <BiUserMinus style="color: var(--el-color-danger)" />
                                </el-icon>
                                <span>{{ $t('session.removeSession') }}</span>
                            </el-dropdown-item>
                        </el-dropdown-menu>
                    </template>
                </el-dropdown>
            </el-button-group>
        </div>
    </div>

    <!-- Dialogs -->
    <div @mousedown.stop>
        <el-dialog v-model="noteDialogVisible" :title="`Notes for「${noteKeyword}」`" width="500" align-center draggable
            :close-on-click-modal="false">
            <el-input v-model="noteContent" class="note-content-input" autocomplete="off" type="textarea"
                :autosize="{ minRows: 5, maxRows: 9 }" />
            <template #footer>
                <div class="dialog-footer">
                    <el-popconfirm confirm-button-text="Delete" confirm-button-type="danger" cancel-button-text="Cancel"
                        :icon="Delete" icon-color="#FF4949" title="Delete this note?" @confirm="deleteNote">
                        <template #reference>
                            <el-button :icon="Delete" type="danger">Delete</el-button>
                        </template>
                    </el-popconfirm>

                    <el-button @click="noteDialogVisible = false">Cancel</el-button>
                    <el-button type="primary" @click="saveNote">Save</el-button>
                </div>
            </template>
        </el-dialog>

        <el-dialog v-model="favoriteWordsDialogVisible" fullscreen>
            <FavoriteWords :favorite-words-dialog-visible="favoriteWordsDialogVisible" :web-socket="webSocket"
                @update-visible="favoriteWordsDialogVisible = $event" :favorite-words="favoriteWords"
                :folder-name="defaultFolderName" :folder-id="sessionConfig.default_folder.id ?? 0" />
        </el-dialog>

        <el-dialog v-model="settingsDialogVisible" fullscreen>
            <Settings :web-socket="webSocket" :setting-dialog-visible="settingsDialogVisible"
                :session-config="sessionConfig" :folder-words="folderWords" :anki-progress="ankiProgress"
                @update-visible="settingsDialogVisible = $event" />
        </el-dialog>

        <el-dialog v-model="dictDialogVisible" fullscreen>
            <DictSelectAndSortDialog :web-socket="webSocket" :env="env" :dictSSDialogVisible="dictDialogVisible"
                :session-config="sessionConfig" :dicts-info="dictsInfo" :add-dict-msgs="addDictMsgs"
                :refresh-dics-settings-info-flag="refreshDicsSettingsInfoFlag"
                @clear:add-dict-msgs="emit('clear:addDictMsgs')" />
        </el-dialog>
    </div>
</template>

<script lang="ts" setup>
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import type { PropType } from 'vue'

// Icons
import {
    BsPin, BsPinAngleFill, BsHeartFill, BsHeart,
} from 'vue-icons-plus/bs'
import { BiUserCheck, BiUser, BiUserPlus, BiUserMinus } from 'vue-icons-plus/bi'
import { LiaUserEditSolid } from 'vue-icons-plus/lia'
import { PiUserSwitch } from 'vue-icons-plus/pi'
import { ImBooks } from 'vue-icons-plus/im'
import { Setting, Edit, Delete, ArrowLeftBold, ArrowRightBold } from '@element-plus/icons-vue'

// Components
import WordOptionsAutoComplete from '@/components/TitleBar/WordOptionsAutoComplete.vue'
import DictSelectAndSortDialog from '@/components/Dialogs/DictSelectAndSortDialog.vue'
import Settings from '@/views/Settings.vue'
import FavoriteWords from '@/components/Dialogs/FavoriteWords.vue'

// Composables & stores
import { useFolderConfigStore } from '@/stores/folderConfig'
import { useHistoryNavigation } from '@/composables/useHistoryNavigation'
import { useSessionManagement } from '@/composables/useSessionManagement'
import { useWindowPin } from '@/composables/useWindowPin'

// Types
import { SessionWebSocketService } from '@/common/session-websocket-client'
import type {
    WordInfoWithLastSearch,
    FolderWords,
    SessionConfig,
    DictInfo,
    SessionNameId,
} from '@/common/type-interface'

// Props & emits
const props = defineProps({
    webSocket: {
        type: [SessionWebSocketService, null],
        required: true,
    },
    sessionId: {
        type: Number,
        required: true,
    },
    env: {
        type: String,
        required: true,
        default: '',
    },
    sessionsNameId: {
        type: Array as PropType<SessionNameId[]>,
        required: true,
        default: () => [],
    },
    sessionConfig: {
        type: Object as PropType<SessionConfig>,
        required: true,
        default: () => ({}),
    },
    dictsInfo: {
        type: Object as PropType<Record<string, DictInfo>>,
        required: true,
        default: () => ({}),
    },
    folderWords: {
        type: Object as PropType<FolderWords>,
        required: true,
        default: () => ({}),
    },
    leftHistory: {
        type: Boolean,
        required: true,
        default: false,
    },
    searchHistory: {
        type: Array as PropType<WordInfoWithLastSearch[]>,
        required: true,
        default: () => [],
    },
    lastSearchKeyword: {
        type: String,
        required: true,
    },
    hasResultLastSearch: {
        type: Boolean,
        required: true,
        default: false,
    },
    noteContent: {
        type: String,
        required: true,
        default: '',
    },
    isWordFavorited: {
        type: Boolean,
        required: true,
        default: false,
    },
    wordOptions: {
        type: Array as PropType<string[]>,
        default: () => [],
    },
    redirectWord: {
        type: String,
        required: true,
        default: '',
    },
    isPinned: {
        type: Boolean,
        default: true,
    },
    iframeKeydownEvent: {
        type: Object as PropType<unknown>,
        default: null,
    },
    ankiProgress: {
        type: Object,
        required: true,
        default: () => ({}),
    },
    addDictMsgs: {
        type: Array,
        default: () => [],
    },
    refreshDicsSettingsInfoFlag: {
        type: Boolean,
        default: true,
    },
    showPopoverWordOptions: {
        type: Boolean,
        default: true,
    },
})

const emit = defineEmits<{
    (e: 'change:keyword', keyword: string): void
    (e: 'clear:addDictMsgs'): void
}>()

// --- Stores ---
const folderConfigStore = useFolderConfigStore()

// --- Dialog visibility state ---
const noteDialogVisible = ref(false)
const favoriteWordsDialogVisible = ref(false)
const settingsDialogVisible = ref(false)
const dictDialogVisible = ref(false)

const noteKeyword = ref('')
const noteContent = ref(props.noteContent)

// --- History navigation ---
const wsRef = computed(() => props.webSocket)
const configRef = computed(() => props.sessionConfig)
const historyRef = computed(() => props.searchHistory)
const leftHistoryRef = computed(() => props.leftHistory)
const hasResultRef = computed(() => props.hasResultLastSearch)

const {
    redirectHistoryWord,
    handleHistoryBack: goBack,
    handleHistoryForward: goForward,
    canGoBack,
    canGoForward,
} = useHistoryNavigation({
    webSocket: wsRef,
    sessionConfig: configRef,
    searchHistory: historyRef,
    leftHistory: leftHistoryRef,
    hasResultLastSearch: hasResultRef,
})

// --- Session management ---
const sessionIdRef = () => props.sessionId
const envRef = () => props.env

const {
    sessionsNameId,
    handleSessionCommand,
} = useSessionManagement(
    () => props.webSocket,
    sessionIdRef,
    envRef
)

// Sync external sessions list
watch(() => props.sessionsNameId, (val) => {
    sessionsNameId.value = val
}, { immediate: true, deep: true })

// --- Window pin ---
const isPinnedRef = () => props.isPinned

const { showPinButton, togglePin } = useWindowPin({
    sessionId: sessionIdRef,
    isPinned: isPinnedRef,
    sessionConfig: configRef,
    webSocket: wsRef,
})

// --- Favorite logic ---
const showFavoriteTooltip = computed(() => {
    const folderId = props.sessionConfig.default_folder.id
    if (!folderId) return true
    return !folderConfigStore.folderConfig?.folders?.folder_info.some(
        (item) => item.id === folderId
    )
})

const canFavorite = computed(() => {
    return (
        props.lastSearchKeyword !== '' &&
        (props.isWordFavorited || props.hasResultLastSearch || props.noteContent !== '')
    )
})

const defaultFolderName = computed(() => {
    const folderId = props.sessionConfig.default_folder.id
    return (
        folderConfigStore.folderConfig?.folders?.folder_info.find(
            (item) => item.id === folderId
        )?.name || ''
    )
})

const favoriteWords = computed(() => {
    const folderId = props.sessionConfig.default_folder.id ?? 0
    return props.folderWords[folderId] || []
})

const toggleFavorite = (): void => {
    props.webSocket?.sendToggleFavor(
        props.lastSearchKeyword,
        props.sessionConfig.default_folder.id ?? null
    )
}

// --- Note dialog ---
const openNoteDialog = (): void => {
    noteKeyword.value = props.lastSearchKeyword
    noteContent.value = props.noteContent
    noteDialogVisible.value = true
}

const saveNote = (): void => {
    if (!noteContent.value.trim()) return
    props.webSocket?.sendSaveWordNote(noteKeyword.value, noteContent.value)
    noteDialogVisible.value = false
}

const deleteNote = (): void => {
    props.webSocket?.sendDeleteWordNote(noteKeyword.value)
    noteDialogVisible.value = false
}

watch(noteDialogVisible, (visible) => {
    props.webSocket?.sendNoteIsEditing(visible)
})

// --- Favorite words dialog ---
watch(favoriteWordsDialogVisible, (visible) => {
    if (visible) {
        const folderId = props.sessionConfig.default_folder.id ?? 0
        props.webSocket?.sendFavoriteWordsRequest(folderId)
    }
})

// --- Keyboard shortcuts from iframe ---
const handleKeydownData = (data: { key: string; altKey: boolean; metaKey: boolean }): void => {
    if (data.key === '/' && data.metaKey) {
        favoriteWordsDialogVisible.value = !favoriteWordsDialogVisible.value
    } else if (data.key === 'ArrowLeft' && data.altKey) {
        goBack()
    } else if (data.key === 'ArrowRight' && data.altKey) {
        goForward()
    }
}

watch(() => props.iframeKeydownEvent, (event) => {
    if (event) handleKeydownData(event as any)
})

// --- Global keyboard ---
const handleGlobalKeydown = (e: KeyboardEvent): void => {
    handleKeydownData({
        key: e.key,
        altKey: e.altKey,
        metaKey: e.metaKey,
    })
}

onMounted(() => {
    window.addEventListener('keydown', handleGlobalKeydown)
})

onUnmounted(() => {
    window.removeEventListener('keydown', handleGlobalKeydown)
})
</script>

<style scoped>
.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
}

:deep(.is-active){
    background-color: var(--el-color-primary-light-8);
}
</style>