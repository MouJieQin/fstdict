<template>
    <el-container>
        <el-header height="var(--header-height)" id="fstdict-header" class="fstdict-header">
            <Titlebar :webSocket="webSocket as SessionWebSocketService" :sessionId="sessionId" :env="envFromRoute"
                :isWordFavorited="isWordFavorited" :sessionConfig="sessionConfig as SessionConfig"
                :folderWords="folderWords" :leftHistory="leftHistory" :searchHistory="searchHistory"
                :isPinned="isFloatingWindowPinned" :lastSearchKeyword="lastSearchKeyword"
                :hasResultLastSearch="hasResultLastSearch" :noteContent="noteContent" :wordOptions="wordOptions"
                :redirectWord="redirectWord" @change:keyword="handleChangeKeyword"
                :iframeKeydownEvent="iframeKeydownEvent" :ankiProgress="ankiProgress" :addDictMsgs="addDictMsgs"
                :refreshDicsSettingsInfoFlag="refreshDicsSettingsInfoFlag" />
        </el-header>
        <el-main class="no-padding-main">
            <el-splitter>
                <el-splitter-panel :size="wordOptionsSize" @update:size="handlePanelResize">
                    <div class="word-options">
                        <WordOptions :webSocket="webSocket as SessionWebSocketService"
                            :sessionConfig="sessionConfig as SessionConfig" :wordOptions="wordOptions"
                            :searchHistory="searchHistory" :keyword="keyword" />
                    </div>
                </el-splitter-panel>
                <el-splitter-panel :min="400">
                    <div class="word-detail"
                        :class="{ 'anki-mode': envFromRoute === 'anki', 'not-anki-mode': envFromRoute !== 'anki' }">
                        <el-collapse class="sticky-collapse" expand-icon-position="left" v-model="activeNames">
                            <el-collapse-item v-if="noteContent" title="我的笔记" name="我的笔记" :isActive="true"
                                class="dict-iframe-container">
                                <template #icon="{ isActive }">
                                    <el-icon v-show="!isActive" class="el-collapse-item__arrow">
                                        <CaretRight />
                                    </el-icon>
                                    <el-icon v-show="isActive" class="el-collapse-item__arrow">
                                        <CaretBottom />
                                    </el-icon>
                                    <BiSolidBookBookmark size="35" />
                                </template>
                                <div class="markdown-note-content" v-html="md.render(noteContent)"></div>
                            </el-collapse-item>
                            <div v-for="(result, dictName) in lookupKeywordResult" :key="dictName">
                                <el-collapse-item :id="`dict-iframe-container-${dictName}`"
                                    class="dict-iframe-container" :title="dictName" :name="dictName" :isActive="true">
                                    <template #icon="{ isActive }">
                                        <el-icon v-show="!isActive" class="el-collapse-item__arrow">
                                            <CaretRight />
                                        </el-icon>
                                        <el-icon v-show="isActive" class="el-collapse-item__arrow">
                                            <CaretBottom />
                                        </el-icon>
                                        <el-image :src="getDictIcon(dictName)" class="collapse-custom-icon">
                                            <template #error>
                                                <BiSolidBookBookmark size="35" />
                                            </template>
                                        </el-image>
                                    </template>

                                    <div v-for="(html, index) in result" :key="html">
                                        <div class="simple-divider"></div>
                                        <DictIframe :dictionary-name="dictName" :index="index" :html="html"
                                            :css-urls="dictsInfo[dictName].css" :js-urls="dictsInfo[dictName].js"
                                            :base-path="dictsInfo[dictName].data"
                                            :dictionary-root="dictsInfo[dictName].root"
                                            :isDark="systemConfigStore.isDark" @entry-click="handleEntryClick"
                                            @keydown="handleIframeKeydown" />
                                    </div>
                                </el-collapse-item>
                            </div>
                        </el-collapse>
                        <div v-show="!keyword && !hasResultLastSearch">
                            <p class="dict-homepage-type-p">Type a word to look up in…</p>
                            <br />
                            <div v-for="dictSetting in sessionDictsSettingInfo" :key="dictSetting.name">
                                <p class="dict-homepage-dict-p" v-show="dictSetting.is_enabled">{{
                                    dictSetting.name }}</p>
                            </div>
                        </div>
                        <div v-show="keyword && lastSearchKeyword && !hasResultLastSearch">
                            <p class="dict-homepage-type-p">No results found for 「{{ lastSearchKeyword }}」 in…</p>
                            <br />
                            <div v-for="dictSetting in sessionDictsSettingInfo" :key="dictSetting.name">
                                <p class="dict-homepage-dict-p" v-show="dictSetting.is_enabled">{{
                                    dictSetting.name }}</p>
                            </div>
                        </div>
                    </div>
                    <el-dropdown placement="bottom-end" @command="handleDropdownCommand">
                        <el-button text class="locate-dict-button" circle bg>
                            <el-icon class="el-icon--right">
                                <MoreFilled />
                            </el-icon>
                        </el-button>
                        <template #dropdown>
                            <el-dropdown-menu>
                                <div v-for="(result, dictName) in lookupKeywordResult" :key="dictName">
                                    <el-dropdown-item :command="dictName">
                                        <el-image :src="getDictIcon(dictName)" class="dropdown-custom-icon">
                                            <template #error>
                                                <BiSolidBookBookmark :size="25" />
                                            </template>
                                        </el-image>
                                        {{ dictName }}
                                    </el-dropdown-item>
                                </div>
                            </el-dropdown-menu>
                        </template>
                    </el-dropdown>
                </el-splitter-panel>
            </el-splitter>
        </el-main>
    </el-container>
</template>

<script setup lang="ts">
import { ref, nextTick, onMounted, onUnmounted, onBeforeUnmount, computed, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { BiSolidBookBookmark } from 'vue-icons-plus/bi'
import { CaretRight, CaretBottom, MoreFilled } from '@element-plus/icons-vue'

import { SessionWebSocketService, useSessionWebSocket } from '@/common/session-websocket-client'
import Titlebar from '@/components/TitleBar/TitleBar.vue'
import WordOptions from '@/components/WordOptions.vue'
import DictIframe from '@/components/DictIframe.vue';
import type { DictsInfo, SessionConfig, DictsSettingInfo, WordInfoWithFavoriteAt, FolderWords, WordInfoWithLastSearch } from '@/common/type-interface'
import { useFolderConfigStore, useSystemConfigStore } from '@/stores/stores'
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
const folderConfigStore = useFolderConfigStore()
const webSocket = ref<SessionWebSocketService | null>(null)
// const bodyScrollTimeoutId = ref<number | null>(null)
const keyword = ref('')
const sessionId = ref(-1)
const keywordFromRoute = ref<string>(route.query.keyword as string || '')
const envFromRoute = ref<string>(route.query.env as string || '')
const redirectWord = ref<string>('')
const dictsInfo = ref<DictsInfo>({})
const sessionDictsSettingInfo = ref<DictsSettingInfo>([])
const sessionConfig = ref<SessionConfig>({
    default_folder: { "id": null },
    dictsSettingOptionName: "default",
    default_search_method: { "method": "prefix_search" },
    pin: { "is_pinned": true }
})
const refreshDicsSettingsInfoFlag = ref<boolean>(false)
const lookupKeywordResult = ref<any>(null)
const wordOptions = ref<string[]>([])
const wordOptionsSize = ref<number | string>(0)
const splitterRef = ref<any>(null)

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
const addDictMsgs = ref<any>([])

const isFloatingWindowPinned = ref<boolean>(sessionConfig.value?.pin?.is_pinned || false)

watch(() => sessionConfig.value?.pin?.is_pinned, (newVal) => {
    isFloatingWindowPinned.value = newVal
})

// 1. Sync manual user dragging changes back to the reactive ref state
const handlePanelResize = (size: number) => {
    console.log("size:", size)
    wordOptionsSize.value = size
}

// 2. Programmatically resize panels safely
const resize_wordoptions = async () => {
    console.log("Current tracking size before shift:", wordOptionsSize.value)

    if (Number(wordOptionsSize.value) <= 5) {
        // Update the layout tracking value
        wordOptionsSize.value = 300

        // Forcing component element updates to bypass internal flex caching layers
        await nextTick()
        if (splitterRef.value) {
            // Accessing internal element instance layouts directly to force rendering updates
            const panelEl = splitterRef.value.$el?.querySelector('.el-splitter-panel')
            if (panelEl) {
                panelEl.style.flexBasis = '300px'
            }
        }
    }
}

const setupDicsSettingsInfo = () => {
    if (!sessionConfig.value?.dictsSettingOptionName || !(sessionConfig.value.dictsSettingOptionName in systemConfigStore.systemConfig.dict_set_options)) {
        sessionConfig.value.dictsSettingOptionName = 'default'
    }
    sessionDictsSettingInfo.value = systemConfigStore.systemConfig.dict_set_options[sessionConfig.value.dictsSettingOptionName]
    refreshDicsSettingsInfoFlag.value = !refreshDicsSettingsInfoFlag.value
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
    if (envFromRoute.value === 'anki') {
        document.body.classList.add('anki-mode')
    } else {
        document.body.classList.remove('anki-mode')
    }
    console.log("Current env:", envFromRoute.value)

    setupWebSocket()
    // window.addEventListener('keydown', handleKeydown)
    // window.addEventListener('scroll', handleScroll)
})

onUnmounted(() => {
    // 移除滚动事件监听器，防止内存泄漏
    // window.removeEventListener('scroll', handleScroll)
})

router.beforeEach(async (__, _, next) => {
    // 关闭 WebSocket
    webSocket.value?.close()
    next()
})

onBeforeUnmount(() => {
    document.title = 'FstDict'
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
            resize_wordoptions()
            console.log('keyword_options_search:', wordOptions.value)
            break
        case 'lookup_keyword_request':
            redirectWord.value = message.data.keyword
            console.log('lookup_keyword_request:', message.data)
            break
        case 'word_note':
            if (message.data.keyword === lastSearchKeyword.value) {
                noteContent.value = message.data.note || ''
            }
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
        case 'folder_config':
            folderConfigStore.setFolderConfig(message.data)
            console.log('folder_config:', message.data)
            break
        case 'system_config':
            systemConfigStore.setSystemConfig(message.data.system_config)
            setupDicsSettingsInfo()
            console.log('system_config:', message.data.system_config)
            break;
        case 'close_fixed_window':
            handleCloseFixedWindow(message)
            break
        case 'anki_progress':
            ankiProgress.value[message.deck_name] = message.data
            break
        case 'add_dictionary':
            addDictMsgs.value = message.data.msgs
            break
        case 'error_session_not_exist':
            router.push('/')
            break
    }
}

const getDictIcon = (dictName: string) => {
    const item = sessionDictsSettingInfo.value?.find((item: any) => item.name === dictName)
    return item ? item.cover_url : ''
}

const handleLookupKeyword = (data: any) => {
    if (envFromRoute.value === 'anki') {
        window.scrollTo(0, 0)
    } else {
        document.querySelector('.word-detail').scrollTo(0, 0)
    }
    const keyword = data.keyword
    document.title = keyword || 'FstDict'
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
    setupDicsSettingsInfo()
    if (message.data.is_right_after_connection) {
        if (envFromRoute.value === 'iwin') {
            webSocket.value?.sendFloatingWindowPinClick(sessionId.value, sessionConfig.value?.pin?.is_pinned || false)
        }
        if (keywordFromRoute.value) {
            webSocket.value?.sendLookupKeywordRequest(keywordFromRoute.value)
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

const handleDropdownCommand = (dictName: string) => {
    const element = document.getElementById(`dict-iframe-container-${dictName}`)
    if (!element) return

    // 1. 先展开词典
    if (!(dictName in activeNames.value)) {
        activeNames.value.push(dictName)
    }

    // 2. 等 Vue DOM 更新 + 浏览器重排完成后再滚动
    nextTick(() => {
        // 获取滚动容器（你的 el-main，替换成你实际的 class/id）
        const scrollContainer = document.querySelector('.word-detail') as HTMLElement
        if (!scrollContainer) return

        // 计算元素相对滚动容器的位置
        const containerRect = scrollContainer.getBoundingClientRect()
        const elementRect = element.getBoundingClientRect()

        const targetScrollTop =
            scrollContainer.scrollTop +
            (elementRect.top - containerRect.top)

        // 执行滚动
        scrollContainer.scrollTo({
            top: targetScrollTop,
            behavior: 'instant'
        })
    })
}

// const handleDropdownCommand = (dictName: string) => {

// 核心：给元素设置顶部滚动边距 = 标题高度
// element.style.scrollMarginTop = '68px';
//     element.scrollIntoView({ behavior: 'instant', block: 'start' })
//     // element.scrollBy(0, 0)
//     if (!(dictName in activeNames.value)) {
//         activeNames.value.push(dictName)
//     }
// }
// }

</script>

<style scoped>
:deep(.no-padding-main) {
    padding: 0;
    /* 直接清除全部内边距 */
    /* 只清除左右保留上下间距就写 padding: 20px 0; */
    flex: 1;
    overflow-y: auto;
}

/* 自定义图标样式 */
:deep(.collapse-custom-icon) {
    width: 2rem;
    height: 2rem;
    margin-right: 8px;
    /* 图标和文字之间的间距 */
    vertical-align: middle;
}

:deep(.el-collapse-item__arrow) {
    /* width: 2rem; */
    /* height: 2rem; */
}

/* 必须用 :deep() 深度选择器，因为 el-collapse-item__header 是Element Plus内部元素 */
:deep(.sticky-collapse .el-collapse-item__header) {
    /* 核心：粘性定位 */
    position: sticky;
    top: 0;
    /* 悬浮在距离顶部0px的位置 */

    /* 必须设置背景色，否则会透明看到下面的内容 */
    background-color: var(--el-bg-color);
    /* 使用Element Plus主题变量，自动适配深色模式 */

    /* 确保悬浮在所有内容（包括iframe）的最上层 */
    /* z-index: 1; */

    /* 可选：添加阴影，增强悬浮层次感 */
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);

    /* 可选：调整内边距，让标题更美观 */
    padding-right: 20px;
}

/* 可选：去掉第一个折叠项的顶部边框，更美观 */
:deep(.sticky-collapse .el-collapse-item:first-child .el-collapse-item__header) {
    border-top: none;
}
</style>