<template>
    <!-- 自定义 macOS 标题栏（仅 macOS 显示，包含 Pin 置顶按钮） -->
    <div>
        <div class="floating-window-titlebar">
            <div @mousedown="preventDrag = true" @mouseup="preventDrag = false">
                <WordOptionsAutoComplete :webSocket="props.webSocket" :env="props.env"
                    :redirectWord="props.redirectWord" :redirectHistoryWord="redirectHistoryWord"
                    :wordOptions="props.wordOptions" :sessionConfig="props.sessionConfig" :searchHistory="searchHistory"
                    @change:keyword="keywordChange" />
            </div>
            <el-button-group class="floating-window-titlebar-button-container" @mousedown="preventDrag = true"
                @mouseup="preventDrag = false">
                <el-button :icon="ArrowLeftBold" text @click="handleHistoryBack" class="floating-window-titlebar-button"
                    size="small" :disabled="historyIndex >= searchHistory.length - 1"
                    id="titlebar-history-back-button" />
                <el-button :icon="ArrowRightBold" text @click="handleHistoryForward"
                    class="floating-window-titlebar-button" size="small"
                    :disabled="historyIndex === -1 || historyIndex === 0" id="titlebar-history-forward-button" />
                <el-tooltip v-if="showFavorButtonTooltip" content="请先设置默认收藏夹" trigger="hover">
                    <el-button :icon="BsHeart" text class="floating-window-titlebar-button" size="small" disabled />
                </el-tooltip>
                <el-button v-if="!showFavorButtonTooltip" :icon="props.isWordFavorited ? BsHeartFill : BsHeart" text
                    @click="handleFavorClick" class="floating-window-titlebar-button" size="small"
                    :disabled="!(lastSearchKeyword !== '' && (props.isWordFavorited || props.hasResultLastSearch || props.noteContent !== ''))" />

                <el-button :icon="Edit" text @click="noteDialogVisible = true" class="floating-window-titlebar-button"
                    size="small" :disabled="!(lastSearchKeyword !== '')" />

                <el-button :icon="ImBooks" text id="titlebar-dictss-button"
                    @click="dictSSDialogVisible = !dictSSDialogVisible" class="floating-window-titlebar-button"
                    size="small" />
                <el-button :icon="Setting" text id="titlebar-setting-button"
                    @click="settingDialogVisible = !settingDialogVisible" class="floating-window-titlebar-button"
                    size="small" />

                <el-dropdown id="titlebar-sessions-button" trigger="click" placement="bottom-end"
                    class="floating-window-titlebar-button" @command="handleSessionCommand">
                    <el-button :icon="PiUserSwitch" text size="small" style="font-size: 15px;" />
                    <template #dropdown>
                        <el-dropdown-menu style="max-height: 60vh; overflow-y: auto;">
                            <div v-for="item in props.sessionsNameId" :key="item.id">
                                <el-dropdown-item v-if="item.id === sessionId"
                                    style="background-color: var(--el-color-primary-light-8) !important">
                                    <el-icon style="color:var(--el-color-primary)" size="25">
                                        <BiUserCheck />
                                    </el-icon>
                                    <span>{{ item.name }}</span>
                                </el-dropdown-item>
                            </div>

                            <div v-for="item in props.sessionsNameId" :key="item.id">
                                <el-dropdown-item v-if="item.id != sessionId" :command="{ cmd: 'switch', id: item.id }">
                                    <el-icon>
                                        <BiUser />
                                    </el-icon>
                                    <span>{{ item.name }}</span>
                                </el-dropdown-item>
                            </div>
                            <el-dropdown-item :command="{ cmd: 'create', id: -1 }">
                                <el-icon>
                                    <BiUserPlus style="color: var(--el-color-primary);" />
                                </el-icon>
                                <span>创建新的Session</span>
                            </el-dropdown-item>
                            <el-dropdown-item :command="{ cmd: 'rename', id: -1 }">
                                <el-icon>
                                    <LiaUserEditSolid style="color: var(--el-color-success);" />
                                </el-icon>
                                <span>编辑当前Session</span>
                            </el-dropdown-item>
                            <el-dropdown-item :command="{ cmd: 'remove', id: -1 }">
                                <el-icon>
                                    <BiUserMinus style="color: var(--el-color-danger);" />
                                </el-icon>
                                <span>移除当前Session</span>
                            </el-dropdown-item>
                        </el-dropdown-menu>
                    </template>
                </el-dropdown>

                <el-button v-if="props.env === 'iwin'" :icon="props.isPinned ? BsPinAngleFill : BsPin" text
                    @click="handlePinClick" class="floating-window-titlebar-button" size="small" />
            </el-button-group>
        </div>
    </div>

    <el-dialog v-model="noteDialogVisible" :title="'「' + keywordEditingNote + '」' + '的笔记（markdown）'" width="500"
        align-center draggable :close-on-click-modal="false">
        <el-input class="note-content-input" v-model="noteContent" autocomplete="off" type="textarea"
            :autosize="{ minRows: 5, maxRows: 9 }" />
        <template #footer>
            <div class="dialog-footer">
                <el-popconfirm confirm-button-text="删除" confirm-button-type="danger" cancel-button-text="取消"
                    :icon="Delete" icon-color="#FF4949" title="确定删除笔记吗？" @confirm="handleDeleteNote">
                    <template #reference>
                        <el-button :icon="Delete" type="danger">Delete</el-button>
                    </template>
                </el-popconfirm>

                <el-button @click="noteDialogVisible = false">Cancel</el-button>
                <el-button type="primary" @click="submitNote">
                    Confirm
                </el-button>
            </div>
        </template>
    </el-dialog>

    <!-- </el-dialog> :z-index="10000"> -->
    <el-dialog v-model="favoriteWordsDialogVisible" fullscreen>
        <FavoriteWords :favoriteWordsDialogVisible="favoriteWordsDialogVisible" :webSocket="props.webSocket"
            @update-visible="(visible) => favoriteWordsDialogVisible = visible" :favoriteWords="favoriteWords"
            :folderName="sessionDefaultFolderName" :folderId="props.sessionConfig.default_folder.id" />
    </el-dialog>
    <el-dialog v-model="settingDialogVisible" fullscreen>
        <Settings :webSocket="props.webSocket" :settingDialogVisible="settingDialogVisible"
            :sessionConfig="props.sessionConfig" :folderWords="props.folderWords" :ankiProgress="ankiProgress"
            @update-visible="(visible) => settingDialogVisible = visible">
        </Settings>
    </el-dialog>
    <el-dialog v-model="dictSSDialogVisible" fullscreen>
        <DictSelectAndSortDialog :webSocket="props.webSocket" :env="props.env"
            :dictSSDialogVisible="dictSSDialogVisible" :sessionConfig="props.sessionConfig"
            :addDictMsgs="props.addDictMsgs" :refreshDicsSettingsInfoFlag="props.refreshDicsSettingsInfoFlag"
            @clear:addDictMsgs="() => emits('clear:addDictMsgs')">
        </DictSelectAndSortDialog>
    </el-dialog>
</template>

<script lang="ts" setup>
import { ref, watch, onMounted, computed, onUnmounted } from 'vue'
import type { PropType } from 'vue'
import type { DropdownInstance } from 'element-plus'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import {
    BsPin, BsPinAngleFill, BsHeartFill, BsHeart,
} from 'vue-icons-plus/bs'
import { BiUserCheck, BiUser, BiUserPlus, BiUserMinus } from 'vue-icons-plus/bi'
import { LiaUserEditSolid } from 'vue-icons-plus/lia'
import { PiUserSwitch } from 'vue-icons-plus/pi'
import { ImBooks } from 'vue-icons-plus/im'
import WordOptionsAutoComplete from '@/components/TitleBar/WordOptionsAutoComplete.vue'
import DictSelectAndSortDialog from '@/components/Dialogs/DictSelectAndSortDialog.vue'
import Settings from '@/views/Settings.vue'
import FavoriteWords from '@/components/Dialogs/FavoriteWords.vue'
import { getDictSettingsForLookup, getDefaultSessionConfig } from '@/common/utility'
import { Setting, Edit, Delete, ArrowLeftBold, ArrowRightBold } from '@element-plus/icons-vue'
import { useFolderConfigStore, useSystemConfigStore } from '@/stores/stores'
import { ElMessageBox } from 'element-plus'
import type { WordInfoWithLastSearch, FolderWords, SessionNameId, SessionConfig } from '@/common/type-interface'
import { useRouter } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'

const props = defineProps({
    webSocket: {
        type: [SessionWebSocketService, null],
        required: true
    },
    sessionId: {
        type: Number,
        required: true
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
        type: Object as () => SessionConfig,
        required: true,
        default: () => ({})
    },
    folderWords: {
        type: Object as () => FolderWords,
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
        type: Array,
        default: () => [],
    },
    redirectWord: {
        type: String,
        required: true,
        default: '',
    },
    isPinned: {
        type: Boolean,
        default: true
    },
    iframeKeydownEvent: {
        type: Object as PropType<any | null>,
        default: () => null,
    },
    ankiProgress: {
        type: Object,
        required: true,
        default: () => ({})
    },
    addDictMsgs: {
        type: Array,
        default: () => [],
    },
    refreshDicsSettingsInfoFlag: {
        type: Boolean,
        default: true,
    },
})

const emits = defineEmits<{
    (e: 'change:keyword', keyword: string): void
    (e: 'clear:addDictMsgs'): void
}>()
const router = useRouter()
const sessionsMenuVisble = ref(false)
const dropdownSessions = ref<DropdownInstance>()
const tauriAppWindow = ref<any | null>(null)
const preventDrag = ref(false)
const redirectHistoryWord = ref('')
const keywordEditingNote = ref('')
const favoriteWordsDialogVisible = ref(false)
const dictSSDialogVisible = ref(false)
const settingDialogVisible = ref(false)
import type { ElAutocomplete } from 'element-plus'
const folderConfigStore = useFolderConfigStore()
const noteDialogVisible = ref(false)
const noteContent = ref(props.noteContent)
const historyIndex = ref(-1)
const isHistoryTriggered = ref(false)

const isTauriEnv = computed(() => {
    return props.env === ''
})

const handCreateSession = () => {
    ElMessageBox.prompt('请输入新的Session名字', 'Tip', {
        confirmButtonText: 'OK',
        cancelButtonText: 'Cancel',
        inputValidator: (value: string) => {
            if(!value){
                return "不能为空"
            }
            if (value.length > 30) {
                return "不能超过30个字符"
            }
        }
    })
        .then(({ value }) => {
            props.webSocket?.sendCreateSession(getDefaultSessionConfig(value))
        })
        .catch(() => {
        })
}

const handRenameSession = () => {
    ElMessageBox.prompt('重命名当前Session名字', 'Tip', {
        confirmButtonText: 'OK',
        cancelButtonText: 'Cancel',
        inputValidator: (value: string) => {
            if (value.length > 30) {
                return "不能超过30个字符"
            }
        }
    })
        .then(({ value }) => {
            props.webSocket?.sendRenameSession(value)
        })
        .catch(() => {
        })
}

const handRemoveSession = () => {
    ElMessageBox.confirm(
        `Are you sure you want to remove the Current Session?`,
        'Warning',
        {
            confirmButtonText: 'OK',
            cancelButtonText: 'Cancel',
            type: 'warning',
            center: true,
        }
    )
        .then(() => {
            props.webSocket?.sendRemoveSession(props.SessionId)
            redirectSession(1)
        })
        .catch(() => {
        })
}

const redirectSession = (sessionId: number) => {
    if (isTauriEnv && sessionId != 1) {
        const systemConfigStore = useSystemConfigStore();
        let localSystemConfig = JSON.parse(JSON.stringify(systemConfigStore.systemConfig))
        localSystemConfig.app.session.id = sessionId
        props.webSocket.sendUpdateSystemConfig(localSystemConfig)
    }
    router.push({
        path: `/dict/${sessionId}`,
        query: { env: props.env }
    })
}


const handleSessionCommand = (command: { cmd: string, id: number }) => {
    if (command.cmd === 'switch') {
        redirectSession(command.id)
    } else if (command.cmd === 'create') {
        handCreateSession()
    }
    else if (command.cmd === 'rename') {
        handRenameSession()
    }
    else if (command.cmd === 'remove') {
        handRemoveSession()
    }
}

const handleDeleteNote = () => {
    props.webSocket?.sendDeleteWordNote(keywordEditingNote.value)
    noteDialogVisible.value = false
}

const submitNote = () => {
    if (!noteContent.value.trim()) {
        return
    }
    props.webSocket?.sendSaveWordNote(keywordEditingNote.value, noteContent.value)
    noteDialogVisible.value = false
}

const keywordChange = (newVal) => {
    emits('change:keyword', newVal)
}

watch(() => noteDialogVisible.value, (newVal) => {
    if (newVal) {
        keywordEditingNote.value = props.lastSearchKeyword
        noteContent.value = props.noteContent
        preventDrag.value = true
    } else {
        preventDrag.value = false
    }
    props.webSocket?.sendNoteIsEditing(newVal)
})

watch(() => favoriteWordsDialogVisible.value, (newVal) => {
    if (newVal) {
        props.webSocket?.sendFavoriteWordsRequest(props.sessionConfig.default_folder.id)
    }
})

watch(() => dictSSDialogVisible.value, (newVal) => {
    if (newVal) {
        preventDrag.value = true
    } else {
        preventDrag.value = false
    }
})

watch(() => settingDialogVisible.value, (newVal) => {
    if (newVal) {
        preventDrag.value = true
    } else {
        preventDrag.value = false
    }
})

watch(() => favoriteWordsDialogVisible.value, (newVal) => {
    if (newVal) {
        preventDrag.value = true
    } else {
        preventDrag.value = false
    }
})


watch(() => props.iframeKeydownEvent, (newVal) => {
    if (newVal) {
        handleKeydown(newVal)
    }
})

const showFavorButtonTooltip = computed(() => {
    return !props.sessionConfig.default_folder.id || !folderConfigStore.folderConfig?.folders?.folder_info.some((item) => item.id === props.sessionConfig.default_folder.id)
})

const sessionDefaultFolderName = computed(() => {
    return folderConfigStore.folderConfig?.folders?.folder_info.find((item) => item.id === props.sessionConfig.default_folder.id)?.name || ''
})

const favoriteWords = computed(() => {
    return props.folderWords[props.sessionConfig.default_folder.id] || []
})

const handlePinClick = () => {
    props.webSocket?.sendFloatingWindowPinClick(props.sessionId, !props.isPinned)
}

const handleFavorClick = () => {
    props.webSocket?.sendToggleFavor(props.lastSearchKeyword, props.sessionConfig.default_folder.id)
}

watch(() => props.leftHistory, (newVal) => {
    if (newVal) {
        isHistoryTriggered.value = false
        setTimeout(() => {
            historyIndex.value = props.hasResultLastSearch ? 0 : -1
        }, 100)
    }
})

watch(() => props.searchHistory, () => {
    if (isHistoryTriggered.value) {
        isHistoryTriggered.value = false
        redirectHistoryWord.value = props.searchHistory[historyIndex.value].word
        props.webSocket?.sendLookupKeyword(redirectHistoryWord.value, props.sessionConfig.default_folder.id, getDictSettingsForLookup(props.sessionConfig.dict_setting_option_name), false)
    }
})

const handleHistoryBack = () => {
    if (historyIndex.value < props.searchHistory.length - 1) {
        historyIndex.value += 1
        isHistoryTriggered.value = true
        props.webSocket?.sendSearchHistoryRequest()
    }

}

const handleHistoryForward = () => {
    if (historyIndex.value > 0) {
        historyIndex.value -= 1
        isHistoryTriggered.value = true
        props.webSocket?.sendSearchHistoryRequest()
    }
}

const handleKeydownData = (keyboardEventData: any) => {
    if (keyboardEventData.key === '/' && keyboardEventData.metaKey) {
        favoriteWordsDialogVisible.value = !favoriteWordsDialogVisible.value
    } else if (keyboardEventData.key === 'ArrowLeft' && keyboardEventData.altKey) {
        // arrow left
        handleHistoryBack()
    } else if (keyboardEventData.key === 'ArrowRight' && keyboardEventData.altKey) {
        // arrow right
        handleHistoryForward()
    }
    // else if (keyboardEventData.key === 'p' && keyboardEventData.altKey) {
    //     if (props.env === 'iwin') {
    //         handlePinClick()
    //     }
    // } else if (keyboardEventData.key === 'b' && keyboardEventData.altKey) {
    //     noteDialogVisible.value = !noteDialogVisible.value
    // } else if (keyboardEventData.key === 'f' && keyboardEventData.altKey) {
    //     handleFavorClick()
    // }
}

const handleKeydown = (e: KeyboardEvent) => {
    handleKeydownData(e)
}

const handleTitlebarMouseDown = (e: MouseEvent) => {
    if (e.buttons === 1) {
        // Primary (left) button
        if (preventDrag.value) {
            return
        }
        if (e.detail === 2) {
            e.preventDefault()
            tauriAppWindow.value?.toggleMaximize() // Maximize on double click
        }
        else {
            e.preventDefault()
            tauriAppWindow.value?.startDragging(); // Else start dragging
        }
    }
}

onMounted(() => {
    window.addEventListener('keydown', handleKeydown)
    if (props.env === '') {
        tauriAppWindow.value = getCurrentWindow();
        document.getElementById('fstdict-header')?.addEventListener('mousedown', handleTitlebarMouseDown)
    }
})

onUnmounted(() => {
    window.removeEventListener('keydown', handleKeydown)
    if (props.env === '') {
        document.getElementById('fstdict-header')?.removeEventListener('mousedown', handleTitlebarMouseDown)
    }
})

</script>
