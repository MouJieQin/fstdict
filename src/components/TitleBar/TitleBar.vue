<template>
    <!-- 自定义 macOS 标题栏（仅 macOS 显示，包含 Pin 置顶按钮） -->
    <div id="mxdict-titlebar" class="mxdict-titlebar">
        <div v-show="showTitleBar" class="floating-window-titlebar">
            <div class="floating-window-search-container" @mousedown="preventDrag = true"
                @mouseup="preventDrag = false">
                <el-autocomplete class="floating-window-search" v-model="keyword" :fetch-suggestions="querySearchAsync"
                    placeholder="Search" @select="handleSelect" ref="autoCompleteRef" @keyup.enter="handleEnter"
                    @focus="handleFocus" clearable hide-loading style="font-size: 1rem;">
                    <!-- 前缀插槽：动态图标 + 点击弹出下拉 -->
                    <template #prefix>
                        <SearchMethodSelect
                            :searchMethod="props.sessionConfig.default_search_method?.method || 'prefix_search'"
                            @update-search-method="handleSearchMethodChange" />
                    </template>
                </el-autocomplete>
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

                <el-button :icon="ImBooks" text @click="dictSSDialogVisible = !dictSSDialogVisible"
                    class="floating-window-titlebar-button" size="small" id="titlebar-dictss-button" />
                <el-button :icon="Setting" text id="titlebar-setting-button"
                    @click="settingDialogVisible = !settingDialogVisible" class="floating-window-titlebar-button"
                    size="small" />
                <el-button v-if="props.env === 'iwin'" :icon="props.isPinned ? BsPinAngleFill : BsPin" text
                    @click="handlePinClick" class="floating-window-titlebar-button" size="small" />
            </el-button-group>
        </div>
    </div>

    <el-dialog v-model="noteDialogVisible" :title="'「' + keywordEditingNote + '」' + '的笔记'" width="500" align-center
        draggable :close-on-click-modal="false">
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

    <el-dialog v-model="favoriteWordsDialogVisible" fullscreen :z-index="10000">
        <FavoriteWords :favoriteWordsDialogVisible="favoriteWordsDialogVisible" :webSocket="props.webSocket"
            @update-visible="(visible) => favoriteWordsDialogVisible = visible" :favoriteWords="favoriteWords"
            :folderName="sessionDefaultFolderName" :folderId="props.sessionConfig.default_folder.id" />
    </el-dialog>
    <el-dialog v-model="settingDialogVisible" fullscreen>
        <Settings :webSocket="props.webSocket" :settingDialogVisible="settingDialogVisible"
            :sessionConfig="props.sessionConfig" :folderWords="props.folderWords" :ankiProgress="ankiProgress">
        </Settings>
    </el-dialog>
    <el-dialog v-model="dictSSDialogVisible" fullscreen>
        <DictSelectAndSortDialog :webSocket="props.webSocket" :dictSSDialogVisible="dictSSDialogVisible"
            :sessionConfig="props.sessionConfig"></DictSelectAndSortDialog>
    </el-dialog>
</template>

<script lang="ts" setup>
import { ref, watch, onMounted, computed, onUnmounted } from 'vue'
import type { PropType } from 'vue'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import {
    BsPin, BsPinAngleFill, BsHeartFill, BsHeart,
} from 'vue-icons-plus/bs'
import { ImBooks } from 'vue-icons-plus/im'
import SearchMethodSelect from '@/components/TitleBar/SearchMethodSelect.vue'
import DictSelectAndSortDialog from '@/components/Dialogs/DictSelectAndSortDialog.vue'
import Settings from '@/views/Settings.vue'
import FavoriteWords from '@/components/Dialogs/FavoriteWords.vue'
import { type SessionConfig } from '@/common/type-interface'
import { getDictSettingsForLookup } from '@/common/utility'
import { Setting, Edit, Delete, ArrowLeftBold, ArrowRightBold } from '@element-plus/icons-vue'
import { useSystemConfigStore } from '@/stores/stores'
import type { WordInfoWithLastSearch, FolderWords } from '@/common/type-interface'
import { getCurrentWindow } from '@tauri-apps/api/window';

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
})

const emits = defineEmits<{
    (e: 'change:keyword', keyword: string): void
}>()

const tauriAppWindow = ref<any | null>(null)
const preventDrag = ref(false)
const keyword = ref('')
const keywordEditingNote = ref('')
const favoriteWordsDialogVisible = ref(false)
const dictSSDialogVisible = ref(false)
const settingDialogVisible = ref(false)
import type { ElAutocomplete } from 'element-plus'
const autoCompleteRef = ref<InstanceType<typeof ElAutocomplete> | null>(null)
const systemConfigStore = useSystemConfigStore()
const noteDialogVisible = ref(false)
const noteContent = ref(props.noteContent)
const historyIndex = ref(-1)
const isHistoryTriggered = ref(false)


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

watch(() => keyword.value, (newVal) => {
    emits('change:keyword', newVal)
})

watch(() => noteDialogVisible.value, (newVal) => {
    if (newVal) {
        keywordEditingNote.value = props.lastSearchKeyword
        noteContent.value = props.noteContent
    }
})

watch(() => favoriteWordsDialogVisible.value, (newVal) => {
    if (newVal) {
        props.webSocket?.sendFavoriteWordsRequest(props.sessionConfig.default_folder.id)
    }
})

watch(() => props.iframeKeydownEvent, (newVal) => {
    if (newVal) {
        handleKeydown(newVal)
    }
})


// 用来存储定时器 ID（关键）
let searchTimer: number | null = null


const showTitleBar = computed(() => {
    return !settingDialogVisible.value
})

const showFavorButtonTooltip = computed(() => {
    return !props.sessionConfig.default_folder.id || !systemConfigStore.systemConfig?.folders?.folder_info.some((item) => item.id === props.sessionConfig.default_folder.id)
})

const sessionDefaultFolderName = computed(() => {
    return systemConfigStore.systemConfig?.folders?.folder_info.find((item) => item.id === props.sessionConfig.default_folder.id)?.name || ''
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

const handleSearchMethodChange = (newMethod: string) => {
    if (props.sessionConfig.default_search_method) {
        props.sessionConfig.default_search_method.method = newMethod
    } else {
        props.sessionConfig.default_search_method = { method: newMethod }
    }
    props.webSocket?.sendSessionConfig(props.sessionConfig)
    // 重新触发搜索以应用新的搜索方法
    if (keyword.value.trim()) {
        querySearchAsync(keyword.value, () => { })
    }
}

watch(() => props.wordOptions, () => {
    links.value = loadAll()
    isOptionsLoading.value = false
})

watch(() => props.leftHistory, (newVal) => {
    if (newVal) {
        isHistoryTriggered.value = false
        setTimeout(() => {
            historyIndex.value = props.hasResultLastSearch ? 0 : -1
        }, 100)
    }
})

watch(() => props.searchHistory, () => {
    links.value = loadAll()
    isOptionsLoading.value = false
    if (isHistoryTriggered.value) {
        isHistoryTriggered.value = false
        keyword.value = props.searchHistory[historyIndex.value].word
        props.webSocket?.sendLookupKeyword(keyword.value, props.sessionConfig.default_folder.id, getDictSettingsForLookup(props.sessionConfig.dictsSettingInfo || []), false)
    }
})

watch(() => props.redirectWord, (newVal) => {
    keyword.value = newVal
    lookupKeyword()
})



interface LinkItem {
    value: string
    link: string
}

const links = ref<LinkItem[]>([])

const loadAll = (): LinkItem[] => {
    if (!keyword.value.trim()) {
        return props.searchHistory.map(item => ({
            value: String(item.word),
            link: String(item.word),
        }))
    }
    return props.wordOptions.map(item => ({
        value: String(item),
        link: String(item),
    }))
}

let isOptionsLoading = ref(false)

const querySearchAsync = (queryString: string, cb: (arg: any) => void) => {
    console.log("queryString", queryString)
    if (!keyword.value.trim()) {
        isOptionsLoading.value = true
        props.webSocket?.sendSearchHistoryRequest()
    } else {
        isOptionsLoading.value = true
        props.webSocket?.sendKeywordOptionsSearch(keyword.value, props.sessionConfig.default_search_method.method, getDictSettingsForLookup(props.sessionConfig.dictsSettingInfo || []))
    }
    // 1. 先清除上一次的定时器（核心！）
    if (searchTimer) {
        clearTimeout(searchTimer)
    }

    let timeCounter = 0
    searchTimer = setInterval(() => {
        timeCounter += 1
        if (timeCounter > 100) {
            isOptionsLoading.value = false
            cb(links.value)
            if (searchTimer) {
                clearTimeout(searchTimer)
            }
            return
        }
        console.log("waiting....")
        if (isOptionsLoading.value) {
            return
        }
        isOptionsLoading.value = false
        cb(links.value)
        console.log("links.value", links.value)
        if (searchTimer) {
            clearTimeout(searchTimer)
        }
    }, 50)
}

const lookupKeyword = () => {
    props.webSocket?.sendLookupKeyword(keyword.value.trim(), props.sessionConfig.default_folder.id, getDictSettingsForLookup(props.sessionConfig.dictsSettingInfo || []))
}

const handleEnter = (e: KeyboardEvent) => {
    e.preventDefault()
    if (props.lastSearchKeyword === keyword.value.trim()) {
        return
    }
    lookupKeyword();
    (autoCompleteRef.value as InstanceType<typeof ElAutocomplete> | null)?.close()
}

const handleSelect = (item: Record<string, any>) => {
    lookupKeyword()
    console.log("handleSelect:", item.value)
}

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



const handleFocus = (_: FocusEvent) => {
    // 拿到原生 input 元素并全选
    // (e.target as HTMLInputElement).select()
}

// const handleKeydown = ref((_: KeyboardEvent) => {})

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
    links.value = loadAll()
    window.addEventListener('keydown', handleKeydown)
    if (props.env === '') {
        tauriAppWindow.value = getCurrentWindow();
        document.getElementById('mxdict-titlebar')?.addEventListener('mousedown', handleTitlebarMouseDown)
    }
})

onUnmounted(() => {
    window.removeEventListener('keydown', handleKeydown)
    if (props.env === '') {
        document.getElementById('mxdict-titlebar')?.removeEventListener('mousedown', handleTitlebarMouseDown)
    }
})

</script>
