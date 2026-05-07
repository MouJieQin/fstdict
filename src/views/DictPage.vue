<template>
    <Titlebar :webSocket="webSocket as SessionWebSocketService" :sessionId="sessionId" :env="envFromRoute"
        :isWordFavorited="isWordFavorited" :sessionConfig="sessionConfig as SessionConfig" :folderWords="folderWords"
        :leftHistory="leftHistory" :searchHistory="searchHistory" :isPinned="isFloatingWindowPinned"
        :lastSearchKeyword="lastSearchKeyword" :hasResultLastSearch="hasResultLastSearch" :noteContent="noteContent"
        :wordOptions="wordOptions" :redirectWord="redirectWord" @change:keyword="handleChangeKeyword"
        :iframeKeydownEvent="iframeKeydownEvent" :ankiProgress="ankiProgress" />
    <div class="word-detail" :style="wordDetailDynamicStyle">
        <el-collapse expand-icon-position="left" v-model="activeNames">
            <el-collapse-item v-if="noteContent" title="我的笔记" name="我的笔记" :isActive="true">
                <el-divider style="margin:0 10px;" />
                <div class="markdown-note-content" v-html="md.render(noteContent)"></div>
            </el-collapse-item>
            <div v-for="(result, dictName) in lookupKeywordResult" :key="dictName">
                <el-collapse-item :title="dictName" :name="dictName" :isActive="true"
                    style="font-weight:bold !important;">
                    <div v-for="html in result" :key="html">
                        <el-divider style="margin:0 10px" />
                        <DictIframe :html="html" :css-urls="dictsInfo[dictName].css" :js-urls="dictsInfo[dictName].js"
                            :base-path="dictsInfo[dictName].data" :dictionary-root="dictsInfo[dictName].root"
                            @entry-click="handleEntryClick" @keydown="handleIframeKeydown" />
                    </div>
                </el-collapse-item>
            </div>
        </el-collapse>
        <div v-show="!keyword && !hasResultLastSearch">
            <p class="dict-homepage-type-p">Type a word to look up in…</p>
            <br />
            <div v-for="dictSetting in sessionConfig.dictsSettingInfo" :key="dictSetting.id">
                <p class="dict-homepage-dict-p" v-show="dictSetting.is_enabled">{{
                    dictSetting.name }}</p>
            </div>
        </div>
        <div v-show="keyword && lastSearchKeyword && !hasResultLastSearch">
            <p class="dict-homepage-type-p">No results found for 「{{ lastSearchKeyword }}」 in…</p>
            <br />
            <div v-for="dictSetting in sessionConfig.dictsSettingInfo" :key="dictSetting.id">
                <p class="dict-homepage-dict-p" v-show="dictSetting.is_enabled">{{
                    dictSetting.name }}</p>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, onBeforeUnmount, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'

import { SessionWebSocketService, useSessionWebSocket } from '@/common/session-websocket-client'
import Titlebar from '@/components/TitleBar/TitleBar.vue'
import DictIframe from '@/components/DictIframe.vue';
import type { DictsInfo, SessionConfig, WordInfoWithFavoriteAt, FolderWords, WordInfoWithLastSearch } from '@/common/type-interface'
import { useSystemConfigStore } from '@/stores/stores'
import MarkdownIt from 'markdown-it'
const md = new MarkdownIt(
    {
        breaks: true,    // 单行换行 → <br>
        xhtmlOut: true   // 闭合<br>标签，兼容更好
    }
)


// 路由与状态
const route = useRoute()
const router = useRouter()

const systemConfigStore = useSystemConfigStore()
const webSocket = ref<SessionWebSocketService | null>(null)
// const bodyScrollTimeoutId = ref<number | null>(null)
const keyword = ref('')
const sessionId = ref(-1)
const keywordFromRoute = route.query.keyword as string || ''
const envFromRoute = route.query.env as string || ''
const redirectWord = ref<string>('')
const dictsInfo = ref<DictsInfo>({})
const sessionConfig = ref<SessionConfig>({
    default_folder: { "id": null },
    dictsSettingInfo: [],
    default_search_method: { "method": "prefix_search" },
    pin: { "is_pinned": true }
})
const lookupKeywordResult = ref<any>(null)
const wordOptions = ref<string[]>([])

const activeNames = ref<string[]>([])
const isWordFavorited = ref<boolean>(false)
const lastSearchKeyword = ref<string>('')
const noteContent = ref<string>('')
const hasResultLastSearch = ref<boolean>(false)
const folderWords = ref<FolderWords>({})
const leftHistory = ref<boolean>(false)
const searchHistory = ref<WordInfoWithLastSearch[]>([])
const iframeKeydownEvent = ref<any | null>(null)
const ankiProgress = ref<any>({})

const isFloatingWindowPinned = ref<boolean>(sessionConfig.value?.pin?.is_pinned || false)

const wordDetailDynamicStyle = computed(() => {
    if (envFromRoute === 'anki') {
        return {
            'max-width': '800px',
            'margin': '0 auto',
        }
    } else {
        return {
        }
    }
})


const setupDicsSettingsInfo = () => {
    if (!sessionConfig.value?.dictsSettingInfo) {
        for (const dictName in dictsInfo.value) {
            const dict = dictsInfo.value[dictName]
            sessionConfig.value?.dictsSettingInfo.push({
                id: dictName,
                name: dict.name,
                cover_url: `http://localhost:5959/api/download?path=${dict.cover}`,
                is_enabled: true
            })
        }
    } else {
        for (const dictName in dictsInfo.value) {
            const dict = dictsInfo.value[dictName]
            const dictSetting = sessionConfig.value?.dictsSettingInfo.find(item => item.id === dictName)
            if (!dictSetting) {
                sessionConfig.value?.dictsSettingInfo.push({
                    id: dictName,
                    name: dict.name,
                    cover_url: `http://localhost:5959/api/download?path=${dict.cover}`,
                    is_enabled: true
                })
            }
        }
    }
}

// 初始化WebSocket
const setupWebSocket = () => {
    sessionId.value = Number(route.params.id)
    webSocket.value = useSessionWebSocket(sessionId.value)
    if (webSocket.value) {
        webSocket.value.handleMessage = (message: any) => {
            handleWebSocketMessage(message)
        }
    }
}

// 初始化
onMounted(() => {
    setupWebSocket()
    // window.addEventListener('keydown', handleKeydown)
    window.addEventListener('scroll', handleScroll)
})

onUnmounted(() => {
    // 移除滚动事件监听器，防止内存泄漏
    window.removeEventListener('scroll', handleScroll)
})

router.beforeEach(async (__, _, next) => {
    // 关闭 WebSocket
    webSocket.value?.close()
    next()
})

onBeforeUnmount(() => {
    document.title = 'MxDict'
})

const handleChangeKeyword = (newKeyword: string) => {
    keyword.value = newKeyword
}


// 处理WebSocket消息
const handleWebSocketMessage = (message: any) => {
    switch (message.type) {
        // case 'toggle_float_pin':
        //     handleToggleFloatPin(message)
        //     break
        case 'dict_info':
            dictsInfo.value = message.data
            setupDicsSettingsInfo()
            console.log('dict_info:', dictsInfo.value)
            break
        case 'keyword_options_search':
            wordOptions.value = message.data.options
            console.log('keyword_options_search:', wordOptions.value)
            break
        case 'lookup_keyword_request':
            redirectWord.value = message.data.keyword
            console.log('lookup_keyword_request:', message.data)
            break
        case 'word_note':
            noteContent.value = message.data.note || ''
            break
        case 'lookup_keyword':
            handleLookupKeyword(message.data)
            console.log('lookup_keyword:', message.data)
            break
        case 'session_config':
            handleSessionConfig(message)
            console.log('session_config:', sessionConfig.value)
            break
        case 'toggle_floating_pin':
            handleToggleFloatPin(message)
            break
        case 'toggle_favor':
            console.log('toggle_favor:', message.data)
            handleToggleFavor(message.data)
            break
        case 'favorite_words':
            folderWords.value[message.data.folder_id] = message.data.words
            console.log('favorite_words:', folderWords.value)
            break
        case 'search_history':
            searchHistory.value = message.data.words
            console.log('search_history:', searchHistory.value)
            break
        case 'system_config':
            systemConfigStore.setSystemConfig(message.data)
            console.log('system_config:', message.data)
            break
        case 'close_fixed_window':
            handleCloseFixedWindow(message)
            break
        case 'anki_progress':
            ankiProgress.value[message.deck_name] = message.data
            break
        case 'error_session_not_exist':
            router.push('/')
            break
    }
}

const handleLookupKeyword = (data: any) => {
    const keyword = data.keyword
    document.title = keyword || 'MxDict'
    lastSearchKeyword.value = keyword || ''
    noteContent.value = data.note || ''
    leftHistory.value = data.left_history
    lookupKeywordResult.value = data.result
    hasResultLastSearch.value = data.result !== null && data.result !== undefined && Object.keys(data.result).length !== 0
    console.log('Object.keys(data.result).length:', Object.keys(data.result).length)
    isWordFavorited.value = data.is_word_favorited
    activeNames.value = Object.keys(data.result).map((key) => key)
    activeNames.value.push('我的笔记')
    console.log('lookup_keyword', keyword, data)
}

const handleEntryClick = (entryPath: string) => {
    redirectWord.value = entryPath
    console.log('redirectWord.value:', redirectWord.value)
}

const handleIframeKeydown = (e: any) => {
    console.log('键盘按下:', e.key)
    iframeKeydownEvent.value = e
}


const handleSessionConfig = (message: any) => {
    sessionConfig.value = message.data.config
    if (message.data.is_right_after_connection) {
        if (envFromRoute === 'iwin') {
            webSocket.value?.sendFloatingWindowPinClick(sessionId.value, sessionConfig.value?.pin?.is_pinned || false)
        }
        if (keywordFromRoute) {
            webSocket.value?.sendLookupKeywordRequest(keywordFromRoute)
        }
    }
}

const handleToggleFloatPin = (message: any) => {
    isFloatingWindowPinned.value = message.data.is_pinned
    if (sessionConfig.value?.pin) {
        if (sessionConfig.value.pin.is_pinned === message.data.is_pinned) {
            return
        }
        sessionConfig.value.pin.is_pinned = message.data.is_pinned
    } else {
        sessionConfig.value.pin = { "is_pinned": message.data.is_pinned }
    }
    webSocket.value?.sendSessionConfig(sessionConfig.value)
}

const handleCloseFixedWindow = (message: any) => {
    console.log('close_fixed_window:', message.data)
}


const handleToggleFavor = (data: any) => {

    isWordFavorited.value = data.is_word_favorited
    if (!isWordFavorited.value) {
        // delete the word in folderWords.value
        folderWords.value[data.folder_id] = folderWords.value[data.folder_id].filter((item: WordInfoWithFavoriteAt) => item.word !== data.keyword)
    }
}

const handleScroll = () => {
    // autoHideScrollbar()
}

</script>