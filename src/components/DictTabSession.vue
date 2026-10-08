<!--
  DictTabSession.vue
  ------------------------------------------------------------------
  One dictionary TAB = one independent dictionary session.
  This component is the "session engine": it owns
    - its own WebSocket connection (bound to props.sessionId)
    - its own WordOptions panel
    - its own DictResultsPanel
  i.e. everything the original single-page DictPage had, minus the shared
  chrome (TitleBar / tab bar / window-level concerns live in DictPage).
  All per-tab runtime state lives in a single reactive `controller`
  object (plain fields, no refs). The controller is registered into the
  dictTabs store so the shared TitleBar can read the ACTIVE tab's data
  through store.activeController.
  Parent communication (via emits):
    create-session(sessionId)  - backend asked us to open another session
    session-error(tabId)       - this session no longer exists on the backend
    redirect-session(sessionId)- window config says we should switch sessions
  Every tab stays MOUNTED (DictPage toggles visibility with v-show), so:
  - background tabs keep receiving WebSocket updates,
  - iframes and scroll positions are preserved without serialization.
-->
<template>
    <el-container class="tab-session">
        <el-main style="padding: 0">
            <el-splitter ref="splitterRef" @resize-start="onSplitterResizeStart" @resize-end="onSplitterResizeEnd">
                <!-- Word options panel (independent per tab) -->
                <el-splitter-panel v-if="!showPopover" :min="wordOptionsMin" :size="wordOptionsSize"
                    @update:size="handlePanelResize" collapsible>
                    <div class="word-options">
                        <WordOptions :web-socket="controller.webSocket" :session-config="controller.sessionConfig"
                            :word-options="controller.wordOptions" :search-history="controller.searchHistory"
                            :keyword="controller.keyword" />
                    </div>
                </el-splitter-panel>
                <!-- Results panel (independent per tab) -->
                <el-splitter-panel :min="400">
                    <DictResultsPanel :controller="controller" :env="env" :tab-id="props.tabId"
                        @context-menu="emit('context-menu', $event)" />
                </el-splitter-panel>
            </el-splitter>
        </el-main>
        <el-aside :width="controller.showConfigPanel ? '100%' : '0px'" class="session-config-aside">
            <SessionSetting :web-socket="controller.webSocket" :session-config="controller.sessionConfig"
                @toggle:config-panel="emit('toggle:config-panel')" />
        </el-aside>
    </el-container>
</template>
<script setup lang="ts">
import { reactive, ref, watch, nextTick, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
// Components
import WordOptions from '@/components/WordOptions.vue'
import DictResultsPanel from '@/components/DictResultsPanel.vue'
import SessionSetting from '@/views/SessionSetting.vue'
// WebSocket & stores
import { useSessionWebSocket } from '@/common/session-websocket-client'
import {
    useFolderConfigStore,
    useDictConfigStore,
    useSystemConfigStore,
} from '@/stores'
import { useDictTabsStore } from '@/stores/dictTabs'
import type { TabController } from '@/stores/dictTabs'
import { getDefaultSessionConfig } from '@/common/utility'
// Constants & i18n
import { ENV, TAURI_CMD } from '@/common/constants'
import { setAppLocale } from '@/i18n'
const props = defineProps<{
    tabId: string
    /** Backend session this tab binds to (null => idle tab, no WebSocket). */
    sessionId: number | null
    env: string
    /** Keyword passed in the route query (deep-link lookup right after connect). */
    initialKeyword: string
    /** Narrow-window mode: hide the word-options panel (owned by the parent). */
    showPopover: boolean
}>()
const emit = defineEmits<{
    (e: 'toggle:config-panel'): void
    (e: 'create-session', sessionId: number): void
    (e: 'session-error', tabId: string): void
    (e: 'redirect-session', sessionId: number): void
    (e: 'context-menu', payload: { selectedText: string; x: number; y: number }): void
}>()
// --- Stores ---
const systemConfigStore = useSystemConfigStore()
const dictConfigStore = useDictConfigStore()
const folderConfigStore = useFolderConfigStore()
const dictTabsStore = useDictTabsStore()
// --- Single source of truth for this tab's runtime state ---
const controller = reactive<TabController>({
    webSocket: null,
    sessionId: null,
    keyword: '',
    lastSearchKeyword: '',
    hasResultLastSearch: false,
    redirectWord: '',
    lookupResults: {},
    noteContent: '',
    leftHistory: false,
    isWordFavorited: false,
    lookupSeq: 0,
    showConfigPanel: false,
    sessionConfig: getDefaultSessionConfig('default'),
    dictsInfo: {},
    sessionDictsSettingInfo: [],
    sessionsNameId: [],
    refreshDicsSettingsInfoFlag: false,
    folderWords: {},
    searchHistory: [],
    wordOptions: [],
    iframeKeydownEvent: null,
    ankiProgress: {},
    addDictMsgs: [],
})
// --- Splitter sizing (local UI state of this tab) ---
const wordOptionsSize = ref<number | string>(0)
const splitterRef = ref<any>(null)
/** Minimum width of the word-options panel (kept 100px while dragging). */
const wordOptionsMin = ref(100)
/** Duration of a programmatic resize animation, in ms (matches the CSS). */
const ANIMATION_MS = 240
/**
 * If the user drags the word-options panel narrower than this many pixels,
 * the drag is finished and the panel folds away automatically. Tune freely.
 */
const RESIZE_COLLAPSE_THRESHOLD_PX = 101
/** The rendered `.el-splitter-panel` element of the word-options panel. */
const wordOptionsPanelEl = (): HTMLElement | null =>
    splitterRef.value?.$el?.querySelector('.el-splitter-panel') ?? null
/**
 * Temporarily enable a CSS transition on the panel so programmatic resizes
 * (expand / collapse) ease smoothly. The class is removed shortly after, so
 * manual bar dragging stays 1:1 with the pointer (no rubber-band lag).
 */
const withPanelAnimation = (): void => {
    const panelEl = wordOptionsPanelEl()
    if (!panelEl) return
    panelEl.classList.add('panel-animating')
    window.setTimeout(() => panelEl.classList.remove('panel-animating'), ANIMATION_MS + 80)
}
const handlePanelResize = (size: number | string): void => {
    wordOptionsSize.value = size
}
const expandWordOptions = async (): Promise<void> => {
    if (props.showPopover) return // options live in the TitleBar popover in narrow mode
    if (Number(wordOptionsSize.value) <= 5) {
        const panelWidth = getComputedStyle(document.documentElement)
            .getPropertyValue('--word-options-panel-width')
            .trim() || '300px'
        withPanelAnimation()
        wordOptionsSize.value = panelWidth
        await nextTick()
    }
}
/**
 * Smoothly fold the word-options panel to exactly 0px.
 * `min="100"` would clamp a `size: 0` prop back to 100 (the panel's prop
 * watcher clamps on every external size change), so we temporarily drop the
 * min for one tick and restore it after the animation finishes.
 */
const collapseWordOptions = (): void => {
    if (wordOptionsMin.value !== 0) {
        wordOptionsMin.value = 0
        window.setTimeout(() => {
            wordOptionsMin.value = 100
        }, ANIMATION_MS + 120)
    }
    withPanelAnimation()
    wordOptionsSize.value = 0
}
/**
 * While the splitter bar is being dragged, the webview happily selects a
 * large page region under the pointer. Block text selection document-wide
 * for the whole drag (restored on `resize-end`).
 */
const onSplitterResizeStart = (): void => {
    document.body.classList.add('dict-resizing')
}
/**
 * Restore selection after the drag. As a safety net, if the panel ended up
 * narrower than the threshold (user dragged it (almost) shut), stop the
 * drag by folding the panel away entirely.
 */
const onSplitterResizeEnd = (): void => {
    document.body.classList.remove('dict-resizing')
    const panelEl = wordOptionsPanelEl()
    if (!panelEl) return
    const widthPx = panelEl.getBoundingClientRect().width
    if (widthPx > 0 && widthPx < RESIZE_COLLAPSE_THRESHOLD_PX) {
        collapseWordOptions()
    }
}
// --- Dictionary setup ---
const setupDictSettings = (): void => {
    const optionName = controller.sessionConfig.dict_setting_option_name
    const options = dictConfigStore.dictConfig?.dict_set_options
    if (!optionName || !options || !(optionName in options)) {
        controller.sessionConfig.dict_setting_option_name = 'default'
    }
    const currentOption = dictConfigStore.dictConfig?.dict_set_options?.[
        controller.sessionConfig.dict_setting_option_name
    ]
    controller.sessionDictsSettingInfo = currentOption || []
    controller.refreshDicsSettingsInfoFlag = !controller.refreshDicsSettingsInfoFlag
}
const setupOcrLangType = (): void => {
    if (!controller.sessionConfig?.ocr_lang_type) {
        controller.sessionConfig.ocr_lang_type = 'English'
    }
}
// --- WebSocket message handlers ---
const handleDictInfo = (data: any): void => {
    controller.dictsInfo = data
    setupDictSettings()
}
const handleDictConfig = (data: any): void => {
    dictConfigStore.setDictConfig(data.dict_config)
    setupDictSettings()
}
const handleSystemConfig = (data: any): void => {
    systemConfigStore.setSystemConfig(data.system_config)
    // Sync the app language with the backend preference.
    const lang = data.system_config?.appearance?.language
    if (lang) setAppLocale(lang)
}
const handleSessionsNameId = (data: any): void => {
    controller.sessionsNameId = data.sessions_name_id
    const config = systemConfigStore.systemConfig
    let targetId: number | undefined
    if (props.env === ENV.MAIN) {
        targetId = config?.app?.windows?.main?.session_id
    } else if (props.env === ENV.HELPER) {
        targetId = config?.app?.windows?.helper_main?.session_id
    } else if (props.env === ENV.SELECTION) {
        targetId = config?.app?.windows?.helper_selection?.session_id
    }
    if (targetId !== undefined && targetId !== controller.sessionId) {
        // The window config wants a different session -> let the parent switch.
        emit('redirect-session', targetId)
    }
}
/** A lookup result belongs to THIS tab (every tab has its own WebSocket). */
const handleLookupKeyword = (data: any): void => {
    const word = data.keyword || ''
    controller.lastSearchKeyword = word
    controller.noteContent = data.note || ''
    controller.leftHistory = data.left_history
    controller.lookupResults = data.result || {}
    controller.hasResultLastSearch = data.result && Object.keys(data.result).length > 0
    controller.isWordFavorited = data.is_word_favorited
    controller.lookupSeq += 1 // signals the results panel to reset scroll
    // The tab label follows the searched word.
    dictTabsStore.setTabTitle(props.tabId, word)
}
const handleToggleFavor = (data: any): void => {
    controller.isWordFavorited = data.is_word_favorited
    if (!data.is_word_favorited) {
        const folderId = data.folder_id
        if (controller.folderWords[folderId]) {
            controller.folderWords[folderId] = controller.folderWords[folderId].filter(
                (item: any) => item.word !== data.keyword
            )
        }
    }
}
const handleSessionConfig = (message: any): void => {
    controller.sessionConfig = message.data.config
    setupDictSettings()
    setupOcrLangType()
    if (message.data.is_right_after_connection) {
        console.log("controller.lastSearchKeyword:", controller.lastSearchKeyword)
        if (props.initialKeyword) {
            initialKeywordSent = true
            controller.webSocket?.sendLookupKeywordRequest(props.initialKeyword)
        } else if (controller.lastSearchKeyword) {
            controller.webSocket?.sendLookupKeyword2(controller.lastSearchKeyword, controller.sessionConfig, controller.leftHistory)
        }
    }
}
const handleCgevent = (data: any): void => {
    if (props.env !== ENV.SELECTION) return
    if (data.type === 'kHandlerTextSelection') {
        controller.redirectWord = data.text_selected
    }
}
const handleTauriNotification = async (data: any): Promise<void> => {
    if (props.env !== ENV.HELPER) return
    await invoke('trigger_notification', { message: data.message || '' })
}
const handleSettingClick = async (): Promise<void> => {
    await invoke(TAURI_CMD.SHOW_SETTING_WINDOW)
}
// --- WebSocket setup (one independent connection per tab) ---
const setupWebSocket = (sessionId: number): void => {
    controller.webSocket?.close()
    controller.sessionId = sessionId
    controller.webSocket = useSessionWebSocket(sessionId)
    controller.webSocket.setMessageHandler(handleWebSocketMessage as any)
}
const handleWebSocketMessage = async (message: any): Promise<void> => {
    switch (message.type) {
        case 'dict_info':
            handleDictInfo(message.data)
            break
        case 'keyword_options_search':
            controller.wordOptions = message.data.options
            expandWordOptions()
            break
        case 'lookup_keyword_request':
            controller.redirectWord = message.data.keyword
            break
        case 'word_note':
            if (message.data.keyword === controller.lastSearchKeyword) {
                controller.noteContent = message.data.note || ''
            }
            break
        case 'lookup_keyword':
            handleLookupKeyword(message.data)
            break
        case 'create_session':
            emit('create-session', message.data.session_id)
            break
        case 'session_config':
            handleSessionConfig(message)
            break
        case 'sessions_name_id':
            handleSessionsNameId(message.data)
            break
        case 'toggle_favor':
            handleToggleFavor(message.data)
            break
        case 'favorite_words':
            controller.folderWords[message.data.folder_id] = message.data.words
            break
        case 'search_history':
            controller.searchHistory = message.data.words
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
            controller.ankiProgress[message.deck_name] = message.data
            break
        case 'add_dictionary':
            controller.addDictMsgs.push(message.data)
            break
        case 'cgevent':
            handleCgevent(message.data)
            break
        case 'tauri_notification':
            await handleTauriNotification(message.data)
            break
        case 'error_session_not_exist':
            emit('session-error', props.tabId)
            break
    }
}
// --- Lifecycle ---
/**
 * Guards the per-tab "initial keyword" deep link so it is sent exactly once
 * per session bind: normally the backend's session_config (sent right after
 * the connection opens, is_right_after_connection=true) triggers it; the
 * ws-open watcher below is a fallback for backends that do not set that flag.
 */
let initialKeywordSent = false
// Connect as soon as a session id is available (initial or assigned later).
// When the id CHANGES (route-driven rebind), drop the old connection and
// open a fresh one; the deep-link guard is reset for the new session.
watch(
    () => props.sessionId,
    (id) => {
        initialKeywordSent = false
        if (id != null) setupWebSocket(id)
    },
    { immediate: true }
)
// Fallback deep link: if the WebSocket reaches OPEN but the backend never
// sent session_config with is_right_after_connection, send the tab's
// initial keyword ourselves (once, after a short grace period).
watch(
    () => controller.webSocket?.status.value,
    (status) => {
        if (status !== 'open' || initialKeywordSent || !props.initialKeyword) return
        setTimeout(() => {
            if (!initialKeywordSent && props.initialKeyword) {
                initialKeywordSent = true
                controller.webSocket?.sendLookupKeywordRequest(props.initialKeyword)
            }
        }, 1200)
    }
)
onMounted(() => {
    // Expose this tab's runtime state to the store (shared TitleBar reads it).
    dictTabsStore.registerController(props.tabId, controller)
})
onBeforeUnmount(() => {
    document.body.classList.remove('dict-resizing')
    controller.webSocket?.close()
    dictTabsStore.unregisterController(props.tabId)
})
</script>
<style scoped>
/*
 * Smooth programmatic expand / collapse. The class is added only for the
 * brief moment of a programmatic resize, so manual bar dragging stays
 * 1:1 with the pointer (flex-basis is re-set on every pointer move).
 */
:deep(.el-splitter-panel.panel-animating) {
    transition: flex-basis 0.24s cubic-bezier(0.22, 0.61, 0.36, 1) !important;
}

/*
 * While the splitter bar is dragged, block page-text selection across the
 * whole document (the webview otherwise selects a big region under the
 * pointer). iframe documents are unaffected (separate documents).
 */
:global(body.dict-resizing) {
    user-select: none !important;
    -webkit-user-select: none !important;
}

:global(body.dict-resizing *) {
    user-select: none !important;
    -webkit-user-select: none !important;
}

/*
 * Constrain the tab shell and clip the splitter panels: the ONLY scroll
 * containers must be the inner el-scrollbar wrap and the word-options
 * virtual list - no phantom panel-level scrollbar, no wheel capture by
 * a middle layer. (Panel heights are driven by the calc(...) rules in
 * components.css, so nothing here fights them.)
 */
.tab-session {
    height: 100%;
}

.tab-session .el-main {
    min-height: 0;
}

.tab-session :deep(.el-splitter-panel) {
    overflow: hidden;
}
</style>