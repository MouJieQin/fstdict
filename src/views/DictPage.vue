<template>
    <el-container>
        <el-header data-tauri-drag-region :height="`var(--header-height)`" id="fstdict-header" class="fstdict-header">
            <TitleBar :web-socket="webSocket" :session-id="sessionId" :env="envFromRoute"
                :is-word-favorited="isWordFavorited" :session-config="sessionConfig" :dicts-info="dictsInfo"
                :sessions-name-id="sessionsNameId" :folder-words="folderWords" :left-history="leftHistory"
                :search-history="searchHistory" :is-pinned="isFloatingWindowPinned"
                :last-search-keyword="lastSearchKeyword" :has-result-last-search="hasResultLastSearch"
                :note-content="noteContent" :word-options="wordOptions" :redirect-word="redirectWord"
                @change:keyword="keyword = $event" @clear:add-dict-msgs="addDictMsgs = []"
                :iframe-keydown-event="iframeKeydownEvent" :anki-progress="ankiProgress" :add-dict-msgs="addDictMsgs"
                :refresh-dics-settings-info-flag="refreshDicsSettingsInfoFlag"
                :show-popover-word-options="showPopoverWordOptions" />
        </el-header>

        <el-main class="no-padding-main">
            <el-splitter ref="splitterRef">

                <el-splitter-panel v-if="!showPopoverWordOptions" :size="wordOptionsSize"
                    @update:size="handlePanelResize">
                    <div class="word-options">
                        <WordOptions :web-socket="webSocket" :session-config="sessionConfig" :word-options="wordOptions"
                            :search-history="searchHistory" :keyword="keyword" />
                    </div>
                </el-splitter-panel>

                <el-splitter-panel :min="400">
                    <div class="word-detail" :class="{
                        'anki-mode': envFromRoute === 'anki',
                        'not-anki-mode': envFromRoute !== 'anki',
                    }">
                        <el-collapse class="sticky-collapse" expand-icon-position="left" v-model="activeNames">
                            <!-- Note panel -->
                            <el-collapse-item v-if="noteContent" :title="$t('dictPage.myNotes')" name="notes"
                                :is-active="true" class="dict-iframe-container">
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

                            <!-- Dictionary result panels -->
                            <el-collapse-item v-for="(htmlList, dictName) in lookupResults" :key="dictName"
                                :id="`dict-iframe-container-${dictName}`" class="dict-iframe-container"
                                :title="dictName" :name="dictName" :is-active="true">
                                <template #icon="{ isActive }">
                                    <el-icon v-show="!isActive" class="el-collapse-item__arrow">
                                        <CaretRight />
                                    </el-icon>
                                    <el-icon v-show="isActive" class="el-collapse-item__arrow">
                                        <CaretBottom />
                                    </el-icon>
                                    <el-image :src="getDictCover(dictName)" class="collapse-custom-icon">
                                        <template #error>
                                            <BiSolidBookBookmark size="35" />
                                        </template>
                                    </el-image>
                                </template>

                                <div v-for="(html, index) in htmlList" :key="index">
                                    <div class="simple-divider"></div>
                                    <DictIframe :dictionary-name="dictName" :index="index" :html="html"
                                        :css-urls="dictsInfo[dictName]?.css || []"
                                        :js-urls="dictsInfo[dictName]?.js || []"
                                        :base-path="dictsInfo[dictName]?.data || ''"
                                        :dictionary-root="dictsInfo[dictName]?.root || ''"
                                        :is-dark="systemConfigStore.isDark" @entry-click="handleEntryClick"
                                        @keydown="handleIframeKeydown" />
                                </div>
                            </el-collapse-item>
                        </el-collapse>

                        <!-- Empty state -->
                        <div v-show="!keyword && !lastSearchKeyword && !hasResultLastSearch" class="empty-state">
                            <p class="dict-homepage-type-p">{{ $t('dictPage.typeToLookup') }}</p>
                            <br />
                            <p v-if="showAddDictInfo" class="dict-homepage-type-p">
                                {{ $t('dictPage.noActiveDicts') }}
                            </p>
                            <p v-for="dict in activeDictionaries" :key="dict.name" class="dict-homepage-dict-p">
                                {{ dict.name }}
                            </p>
                        </div>

                        <div v-show="lastSearchKeyword && !hasResultLastSearch" class="empty-state">
                            <p class="dict-homepage-type-p">
                                {{ $t('dictPage.noResults', { word: lastSearchKeyword }) }}
                            </p>
                            <br />
                            <p v-if="showAddDictInfo" class="dict-homepage-type-p">
                                {{ $t('dictPage.noActiveDicts') }}
                            </p>
                            <p v-for="dict in activeDictionaries" :key="dict.name" class="dict-homepage-dict-p">
                                {{ dict.name }}
                            </p>
                        </div>
                    </div>

                    <!-- Floating locate button -->
                    <el-dropdown placement="bottom-end" @command="scrollToDictionary">
                        <el-button text class="locate-dict-button" circle bg>
                            <el-icon class="el-icon--right">
                                <MoreFilled />
                            </el-icon>
                        </el-button>
                        <template #dropdown>
                            <el-dropdown-menu>
                                <el-dropdown-item v-for="(_, dictName) in lookupResults" :key="dictName"
                                    :command="dictName">
                                    <el-image :src="getDictCover(dictName)" class="dropdown-custom-icon">
                                        <template #error>
                                            <BiSolidBookBookmark :size="25" />
                                        </template>
                                    </el-image>
                                    {{ dictName }}
                                </el-dropdown-item>
                            </el-dropdown-menu>
                        </template>
                    </el-dropdown>
                </el-splitter-panel>
            </el-splitter>
        </el-main>
    </el-container>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { isTauri } from '@tauri-apps/api/core'
import MarkdownIt from 'markdown-it'

// Icons
import { BiSolidBookBookmark } from 'vue-icons-plus/bi'
import { CaretRight, CaretBottom, MoreFilled } from '@element-plus/icons-vue'

// Components
import TitleBar from '@/components/TitleBar/TitleBar.vue'
import WordOptions from '@/components/WordOptions.vue'
import DictIframe from '@/components/DictIframe.vue'

// WebSocket & stores
import { useSessionWebSocket } from '@/common/session-websocket-client'
import {
    useFolderConfigStore,
    useDictConfigStore,
    useSystemConfigStore,
} from '@/stores'
import { getDefaultSessionConfig } from '@/common/utility'

// Types
import type {
    DictsInfo,
    SessionNameId,
    SessionConfig,
    DictsSettingInfo,
    FolderWords,
    WordInfoWithLastSearch,
} from '@/common/type-interface'

import { ENV } from '@/common/constants'

// add import at top
import { setAppLocale } from '@/i18n'


// Markdown renderer
const md = new MarkdownIt({
    breaks: true,
    xhtmlOut: true,
})

// --- Router & route ---
const route = useRoute()
const router = useRouter()

// --- Stores ---
const systemConfigStore = useSystemConfigStore()
const dictConfigStore = useDictConfigStore()
const folderConfigStore = useFolderConfigStore()

// --- Reactive state ---
const webSocket = ref<ReturnType<typeof useSessionWebSocket> | null>(null)
const keyword = ref('')
const sessionId = ref(-1)
const envFromRoute = ref('')
const redirectWord = ref('')

const dictsInfo = ref<DictsInfo>({})
const sessionDictsSettingInfo = ref<DictsSettingInfo>([])
const sessionsNameId = ref<SessionNameId[]>([])
const sessionConfig = ref<SessionConfig>(getDefaultSessionConfig('default'))
const refreshDicsSettingsInfoFlag = ref(false)

const lookupResults = ref<Record<string, string[]>>({})
const wordOptions = ref<string[]>([])
const wordOptionsSize = ref<number | string>(0)
const splitterRef = ref<any>(null)

const activeNames = ref<string[]>([])
const isWordFavorited = ref(false)
const lastSearchKeyword = ref('')
const noteContent = ref('')
const hasResultLastSearch = ref(false)
const folderWords = ref<FolderWords>({})
const leftHistory = ref(false)
const searchHistory = ref<WordInfoWithLastSearch[]>([])
const iframeKeydownEvent = ref<unknown>(null)
const ankiProgress = ref<Record<string, any>>({})
const addDictMsgs = ref<any[]>([])
const showAddDictInfo = ref(false)
const viewportWidth = ref(window.innerWidth)
const showPopoverWordOptions = ref(false)

const isFloatingWindowPinned = computed(
    () => sessionConfig.value?.pin?.is_pinned || false
)

const activeDictionaries = computed(() =>
    sessionDictsSettingInfo.value.filter((d) => d.is_enabled)
)

// --- Helper functions ---
const getDictCover = (dictName: string): string => dictsInfo.value[dictName]?.cover_url || ''

const setShowAddDictInfo = (): void => {
    showAddDictInfo.value = !activeDictionaries.value.length
}

// --- Panel sizing ---
const handlePanelResize = (size: number): void => {
    wordOptionsSize.value = size
}

const expandWordOptions = async (): Promise<void> => {
    if (Number(wordOptionsSize.value) <= 5) {
        const panelWidth = getComputedStyle(document.documentElement).getPropertyValue('--word-options-panel-width').trim()
        wordOptionsSize.value = panelWidth
        await nextTick()
        if (splitterRef.value) {
            const panelEl = splitterRef.value.$el?.querySelector('.el-splitter-panel')
            if (panelEl) {
                panelEl.style.flexBasis = panelWidth
            }
        }
    }
}

// --- Dictionary setup ---
const setupDictSettings = (): void => {
    const optionName = sessionConfig.value.dict_setting_option_name
    const options = dictConfigStore.dictConfig?.dict_set_options

    if (!optionName || !options || !(optionName in options)) {
        sessionConfig.value.dict_setting_option_name = 'default'
    }

    const currentOption = dictConfigStore.dictConfig?.dict_set_options?.[
        sessionConfig.value.dict_setting_option_name
    ]

    sessionDictsSettingInfo.value = currentOption || []
    setShowAddDictInfo()
    refreshDicsSettingsInfoFlag.value = !refreshDicsSettingsInfoFlag.value
}

const setupOcrLangType = (): void => {
    if (!sessionConfig.value?.ocr_lang_type) {
        sessionConfig.value.ocr_lang_type = 'English'
    }
}

// --- WebSocket message handlers ---
const handleDictInfo = (data: any): void => {
    dictsInfo.value = data
    setupDictSettings()
}

const handleDictConfig = (data: any): void => {
    dictConfigStore.setDictConfig(data.dict_config)
    setupDictSettings()
}

// update handleSystemConfig function
const handleSystemConfig = (data: any): void => {
    systemConfigStore.setSystemConfig(data.system_config)
    // sync language preference
    const lang = data.system_config?.appearance?.language
    if (lang) setAppLocale(lang)
}

const handleSessionsNameId = (data: any): void => {
    sessionsNameId.value = data.sessions_name_id

    const env = envFromRoute.value
    const config = systemConfigStore.systemConfig
    let targetId: number | undefined

    if (env === ENV.MAIN) {
        targetId = config?.app?.session?.id
    } else if (env === ENV.HELPER) {
        targetId = config?.ocr?.session?.id
    } else if (env === ENV.SELECTION) {
        targetId = config?.app?.helper_selection?.session?.id
    }

    if (targetId !== undefined && targetId !== sessionId.value) {
        redirectToSession(targetId)
    }
}

const handleLookupKeyword = (data: any): void => {
    if (envFromRoute.value === 'anki') {
        window.scrollTo(0, 0)
    } else {
        document.querySelector('.word-detail')?.scrollTo(0, 0)
    }

    const word = data.keyword
    document.title = word || 'FstDict'

    lastSearchKeyword.value = word || ''
    noteContent.value = data.note || ''
    leftHistory.value = data.left_history
    lookupResults.value = data.result || {}
    hasResultLastSearch.value = data.result && Object.keys(data.result).length > 0
    isWordFavorited.value = data.is_word_favorited

    activeNames.value = Object.keys(data.result || {})
    if (noteContent.value) {
        activeNames.value.unshift('notes')
    }
}

const handleToggleFavor = (data: any): void => {
    isWordFavorited.value = data.is_word_favorited

    if (!isWordFavorited.value) {
        const folderId = data.folder_id
        if (folderWords.value[folderId]) {
            folderWords.value[folderId] = folderWords.value[folderId].filter(
                (item: any) => item.word !== data.keyword
            )
        }
    }
}

const handleSessionConfig = (message: any): void => {
    sessionConfig.value = message.data.config
    setupDictSettings()
    setupOcrLangType()

    if (message.data.is_right_after_connection) {
        if (envFromRoute.value === 'iwin') {
            webSocket.value?.sendFloatingWindowPinClick(
                sessionId.value,
                sessionConfig.value?.pin?.is_pinned || false
            )
        }

        const keywordFromRoute = route.query.keyword as string
        if (keywordFromRoute) {
            webSocket.value?.sendLookupKeywordRequest(keywordFromRoute)
        }
    }
}

const handleToggleFloatPin = (message: any): void => {
    const pinned = message.data.is_pinned

    if (sessionConfig.value?.pin) {
        if (sessionConfig.value.pin.is_pinned === pinned) return
        sessionConfig.value.pin.is_pinned = pinned
    } else {
        sessionConfig.value.pin = { is_pinned: pinned }
    }

    webSocket.value?.sendSessionConfig(sessionConfig.value)
}

const handleCgevent = (data: any): void => {
    if (envFromRoute.value !== ENV.SELECTION) return
    if (data.type === 'kHandlerTextSelection') {
        redirectWord.value = data.text_selected
    }
}

const handleTauriNotification = (data: any): void => {
    if (envFromRoute.value !== ENV.HELPER) return
    import('@tauri-apps/api/core').then(({ invoke }) => {
        invoke('trigger_notification', { message: data.message || '' })
    })
}

// --- WebSocket setup ---
const setupWebSocket = (): void => {
    sessionId.value = Number(route.params.id)
    webSocket.value = useSessionWebSocket(sessionId.value)

    if (webSocket.value) {
        webSocket.value.setMessageHandler(handleWebSocketMessage as any)
    }
}

const handleWebSocketMessage = (message: any): void => {
    switch (message.type) {
        case 'dict_info':
            handleDictInfo(message.data)
            break
        case 'keyword_options_search':
            wordOptions.value = message.data.options
            expandWordOptions()
            break
        case 'lookup_keyword_request':
            redirectWord.value = message.data.keyword
            break
        case 'word_note':
            if (message.data.keyword === lastSearchKeyword.value) {
                noteContent.value = message.data.note || ''
            }
            break
        case 'lookup_keyword':
            handleLookupKeyword(message.data)
            break
        case 'create_session':
            redirectToSession(message.data.session_id)
            break
        case 'session_config':
            handleSessionConfig(message)
            break
        case 'sessions_name_id':
            handleSessionsNameId(message.data)
            break
        case 'toggle_floating_pin':
            handleToggleFloatPin(message)
            break
        case 'toggle_favor':
            handleToggleFavor(message.data)
            break
        case 'favorite_words':
            folderWords.value[message.data.folder_id] = message.data.words
            break
        case 'search_history':
            searchHistory.value = message.data.words
            expandWordOptions()
            break
        case 'folder_config':
            folderConfigStore.setFolderConfig(message.data)
            break
        case 'dict_config':
            handleDictConfig(message.data)
            break
        case 'system_config':
            handleSystemConfig(message.data)
            break
        case 'anki_progress':
            ankiProgress.value[message.deck_name] = message.data
            break
        case 'add_dictionary':
            addDictMsgs.value.push(message.data)
            break
        case 'cgevent':
            handleCgevent(message.data)
            break
        case 'tauri_notification':
            handleTauriNotification(message.data)
            break
        case 'error_session_not_exist':
            router.push('/')
            break
    }
}

// --- Navigation ---
const redirectToSession = (id: number): void => {
    router.push({
        path: `/dict/${id}`,
        query: { env: envFromRoute.value },
    })
}

// --- Iframe events ---
const handleEntryClick = (entryPath: string): void => {
    redirectWord.value = entryPath
}

const handleIframeKeydown = (e: unknown): void => {
    iframeKeydownEvent.value = e
}

const scrollToDictionary = (dictName: string): void => {
    const element = document.getElementById(`dict-iframe-container-${dictName}`)
    if (!element) return

    if (!activeNames.value.includes(dictName)) {
        activeNames.value.push(dictName)
    }

    nextTick(() => {
        const container = document.querySelector('.word-detail') as HTMLElement | null
        if (!container) return

        const containerRect = container.getBoundingClientRect()
        const elementRect = element.getBoundingClientRect()
        const targetTop = container.scrollTop + (elementRect.top - containerRect.top)

        container.scrollTo({ top: targetTop, behavior: 'instant' })
    })
}

// --- Tauri event listeners ---
let unlistenTextSelected: (() => void) | null = null
let unlistenOcrResult: (() => void) | null = null

const setupTauriListeners = async (): Promise<void> => {
    try {
        if (envFromRoute.value === 'selection_float_search' || envFromRoute.value === '') {
            unlistenTextSelected = await listen('cgevent-select', (event) => {
                redirectWord.value = event.payload as string
            })
        }

        if (envFromRoute.value === 'helper_main_tauri' || envFromRoute.value === '') {
            unlistenOcrResult = await listen('cgevent-ocr', (event) => {
                redirectWord.value = event.payload as string
            })
        }
    } catch (error) {
        console.error('Failed to bind Tauri event listeners:', error)
    }
}

// --- Viewport resize ---
const handleResize = (): void => {
    viewportWidth.value = window.innerWidth
}

// --- Lifecycle ---
const initDictPage = async (): Promise<void> => {
    // Apply anki mode class
    if (envFromRoute.value === 'anki') {
        document.body.classList.add('anki-mode')
    } else {
        document.body.classList.remove('anki-mode')
    }

    await setupTauriListeners()
    setupWebSocket()
    window.addEventListener('resize', handleResize)
    showPopoverWordOptions.value = window.innerWidth < 700
}

onMounted(async () => {
    envFromRoute.value = (route.query.env as string) || ''
    await initDictPage()
})

onUnmounted(() => {
    window.removeEventListener('resize', handleResize)
    unlistenTextSelected?.()
    unlistenOcrResult?.()
})

onBeforeUnmount(() => {
    document.title = 'FstDict'
})

// Route change handler
watch(
    () => route.params.id,
    async () => {
        webSocket.value?.close()
        await initDictPage()
    }
)

watch(() => lastSearchKeyword.value, async (val) => {
    if (isTauri()) {
        try {
            await getCurrentWindow().setTitle(val)
        } catch (error) {
            console.error('Failed to set window title:', error)
        }
    }
})

watch(() => viewportWidth.value, (width) => {
    showPopoverWordOptions.value = width < 700
})

// Router guard for cleanup
router.beforeEach(async () => {
    webSocket.value?.close()
    return true
})
</script>

<style scoped>
:deep(.no-padding-main) {
    padding: 0;
    flex: 1;
    overflow-y: auto;
}

:deep(.collapse-custom-icon) {
    flex-shrink: 0;
    width: 2rem;
    height: 2rem;
    margin-right: 8px;
    vertical-align: middle;
}

:deep(.el-collapse-item__arrow) {
    flex-shrink: 0;
}

:deep(.sticky-collapse .el-collapse-item__header) {
    position: sticky;
    top: 0;
    background-color: var(--el-bg-color);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
    padding-right: 20px;
    white-space: nowrap;
    overflow: hidden;
}

:deep(.sticky-collapse .el-collapse-item:first-child .el-collapse-item__header) {
    border-top: none;
}

.empty-state {
    text-align: center;
}
</style>