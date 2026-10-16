<!--
  DictPage.vue
  ------------------------------------------------------------------
  Parent / orchestrator of the multi-tab dictionary page.

  What lives here:
  - the SHARED chrome: TitleBar (bound to the ACTIVE tab's controller),
    the DictTabs tab bar, and the per-tab DictTabSession list (all tabs
    stay mounted, visibility toggled with v-show)
  - window-level concerns: route env, Tauri event listeners (forwarded to
    the active tab), window title, viewport/sidebar behaviour

  What lives in children:
  - DictTabSession  = one tab's full session (own WebSocket, own WordOptions,
                      own results panel; state in its `controller`)
  - DictTabs        = tab strip UI (drag reorder, add/close intents)
  - DictResultsPanel= result area of one tab

  Session orchestration:
  - the initial tab is created from the route (`/dict/:id`)
  - "+" asks the ACTIVE tab's WebSocket to create a backend session; the
    backend answers with a `create_session` message, and the tab that
    received it emits create-session -> a new tab is opened here
-->
<template>
    <div class="common-layout" :class="{ 'is-main': envFromRoute === ENV.MAIN }">
        <el-container :style="{ '--tab-height': `${tabHeight}px` }">
            <el-header :height="`calc(var(--header-height) + var(--tab-height))`" id="fstdict-header"
                class="fstdict-header" :style="{
                    '--header-padding-right': `${headerPaddingRight}px`,
                    '--header-padding-left': `${headerPaddingLeft}px`,
                }">
                <el-container>
                    <!-- Shared title bar: every data prop follows the ACTIVE tab -->
                    <el-header data-tauri-drag-region :height="`var(--header-height)`" class="fstdict-titlebar">
                        <TitleBar :web-socket="activeController?.webSocket ?? null"
                            :session-id="activeController?.sessionId ?? -1" :env="envFromRoute"
                            :keyword-to-show="activeController?.keyword ?? ''"
                            :is-word-favorited="activeController?.isWordFavorited"
                            :session-config="activeController?.sessionConfig ?? defaultSessionConfig"
                            :dicts-info="activeController?.dictsInfo"
                            :sessions-name-id="activeController?.sessionsNameId"
                            :folder-words="activeController?.folderWords" :left-history="activeController?.leftHistory"
                            :search-history="activeController?.searchHistory"
                            :last-search-keyword="activeController?.lastSearchKeyword ?? ''"
                            :has-result-last-search="activeController?.hasResultLastSearch"
                            :note-content="activeController?.noteContent" :word-options="activeController?.wordOptions"
                            :redirect-word="dictTabsStore.redirectWord" @change:keyword="handleTitleBarKeyword"
                            @open-tab-with-keyword="openTabWithKeyword" @redirect-session="handleRedirectSession"
                            @toggle:main-sidebar="emit('toggle:main-sidebar', $event)"
                            @toggle:config-panel="activeController!.showConfigPanel = !activeController!.showConfigPanel"
                            :iframe-keydown-event="dictTabsStore.iframeKeydownEvent"
                            :anki-progress="activeController?.ankiProgress"
                            :refresh-dics-settings-info-flag="activeController?.refreshDicsSettingsInfoFlag"
                            :show-popover-word-options="showPopoverWordOptions" :show-sidebar="showSidebar"
                            :is-main-sidebar-collapsed="isMainSidebarCollapsed" />
                    </el-header>
                    <!-- Tab bar row -->
                    <el-main data-tauri-drag-region style="padding: 0;">
                        <DictTabs @add-tab="requestNewSession" @close-tab="dictTabsStore.closeTab" />
                    </el-main>
                </el-container>
            </el-header>

            <!--
        One full dictionary session per tab; all stay mounted (v-show).
        IMPORTANT: iterate tabsByInsertionOrder here, NOT dictTabsStore.tabs.
        The tab bar reorders `tabs` on drag; if the content v-for followed
        that order, Vue would MOVE each DictTabSession root DOM node, and
        browsers reload every <iframe> when its host node is re-attached
        -> white result area. Insertion order never changes, so content
        DOM nodes are never moved.
      -->
            <el-main class="no-padding-main">
                <el-splitter ref="splitterRef">
                    <el-splitter-panel max="100%">
                        <DictTabSession v-for="tab in tabsByInsertionOrder"
                            v-show="tab.id === dictTabsStore.activeTabId" :key="tab.id" :tab-id="tab.id"
                            :session-id="tab.sessionId" :env="envFromRoute" :initial-keyword="tab.initialKeyword"
                            :initial-keyword-to-show="tab.initialKeywordToShow" :show-popover="showPopoverWordOptions"
                            @toggle:config-panel="activeController!.showConfigPanel = !activeController!.showConfigPanel"
                            @create-session="handleCreateSession" @session-error="handleSessionError"
                            @open-tab-with-keyword="openTabWithKeyword" @activate-session="handleActivateSession"
                            @iframe-keydown="handleIframeKeydown" @context-menu="handleIframeContextMenu" />
                    </el-splitter-panel>
                    <el-splitter-panel max="80%" :size="glossaryPanelSize" collapsible @update:size="handlePanelResize">
                        <GlossaryOptions :active-menu-index="activeMenuIndex"
                            :web-socket="activeController?.webSocket ?? null"
                            :session-config="activeController?.sessionConfig"
                            :keyword="activeController?.lastSearchKeyword" :folder-name="folderName"
                            :favorite-words="favoriteWords" :search-history="activeController?.searchHistory"
                            :note-words="noteWords" />
                    </el-splitter-panel>
                </el-splitter>
            </el-main>
        </el-container>
    </div>
</template>

<script setup lang="ts">
import type { PropType } from 'vue'
import { ref, computed, nextTick, watch, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { platform } from '@tauri-apps/plugin-os'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { i18n } from '@/i18n'
import type { WordInfoWithFavoriteAt, WordInfoWithNoteUpdateAt } from '@/common/type-interface'

// Components
import TitleBar from '@/components/TitleBar/TitleBar.vue'
import DictTabs from '@/components/DictTabs.vue'
import DictTabSession from '@/components/DictTabSession.vue'
import GlossaryOptions from '@/components/GlossaryOptions.vue'

// Stores
import { useDictTabsStore } from '@/stores/dictTabs'

// Constants
import { ENV, TAURI_EVENT, TAURI_CMD, TAB_HEIGHT, MAIN_MENU_INDEX } from '@/common/constants'
import { getDefaultSessionConfig } from '@/common/utility'

const emit = defineEmits<{
    (e: 'toggle:main-sidebar', isCollapsed: boolean): void
}>()

const props = defineProps({
    showSidebar: {
        type: Boolean,
        default: true,
    },
    isMainSidebarCollapsed: {
        type: Boolean,
        default: false,
    },
    activeMenuIndex: {
        type: String,
        default: '',
    },
    noteWords: {
        type: Array as PropType<WordInfoWithNoteUpdateAt[]>,
        required: false,
        default: () => [],
    },
    folderName: {
        type: String,
        default: '',
    },
    favoriteWords: {
        type: Array as PropType<WordInfoWithFavoriteAt[]>,
        required: false,
        default: () => [],
    },
})

// --- Route & store ---
const route = useRoute()
const dictTabsStore = useDictTabsStore()

/** Runtime state of the ACTIVE tab (drives the shared TitleBar). */
const activeController = computed(() => dictTabsStore.activeController)

/**
 * Content-area children in IMMUTABLE insertion order (see template note).
 * The tab bar may reorder tabs on drag, but this list never changes order,
 * so Vue never re-attachs the DictTabSession DOM subtrees (and never
 * reloads the dictionary iframes inside them).
 */
const tabsByInsertionOrder = computed(() => dictTabsStore.tabsByInsertionOrder)

const tabsCount = computed(() => dictTabsStore.tabs.length)

/**
 * Fallback session config for the shared TitleBar while no tab controller
 * is registered yet (first render) - TitleBar reads default_folder.id, so
 * it must always receive a real SessionConfig object, never undefined.
 */
const defaultSessionConfig = getDefaultSessionConfig('default')

// --- Window-level state ---
const envFromRoute = ref('')
const headerPaddingRight = ref(0)
const headerPaddingLeft = ref(0)
const tabHeight = ref(0)

const viewportWidth = ref(window.innerWidth)
const showPopoverWordOptions = ref(false)

// Splitter sizing (local UI state of this tab).
const showGlossaryPanel = ref(false)
const glossaryPanelSize = ref<number | string>(0)
const minGlossaryPanel = ref<number | string>(0)
const splitterRef = ref<any>(null)

const handlePanelResize = async (size: number): Promise<void> => {
    if (size == 0) return
    if (size <= 50) {
        await collpaseGlossaryPanel()
    } else {
        glossaryPanelSize.value = size
    }
}

const resizeGlossaryPanel = async (): Promise<void> => {
    await nextTick()
    if (splitterRef.value) {
        const panelEl = splitterRef.value.$el?.querySelector('.el-splitter-panel')
        if (panelEl) {
            panelEl.style.flexBasis = glossaryPanelSize.value
        }
    }
}

const collpaseGlossaryPanel = async (): Promise<void> => {
    showGlossaryPanel.value = false
    minGlossaryPanel.value = 0
    glossaryPanelSize.value = 0
}

const expandGlossaryPanel = async (): Promise<void> => {
    showGlossaryPanel.value = true

    minGlossaryPanel.value = "100px"
    if (Number(glossaryPanelSize.value) <= 5) {
        glossaryPanelSize.value = 200
        await resizeGlossaryPanel()
    }
}

watch(
    () => props.activeMenuIndex,
    (newIndex) => {
        if (newIndex === MAIN_MENU_INDEX.HISTORY || newIndex === MAIN_MENU_INDEX.NOTES || newIndex.startsWith(MAIN_MENU_INDEX.FLOSSARY_FOLDERS_PREFIX)) {
            expandGlossaryPanel()
        }
        else if (newIndex === MAIN_MENU_INDEX.DICTIONARY) {
            collpaseGlossaryPanel()
        }
    }
)

// --- TitleBar glue ---
/** Typing in the shared search box targets the ACTIVE tab's keyword. */
const handleTitleBarKeyword = (value: string): void => {
    const controller = dictTabsStore.activeController
    if (controller) controller.keyword = value
}

/** Clear the active tab's "add dictionary" toast messages. */
const handleClearAddDictMsgs = (): void => {
    const controller = dictTabsStore.activeController
    if (controller) controller.addDictMsgs = []
}

// --- Session orchestration ---

/**
 * "+" tab button: open a new connection bound to the CURRENT ACTIVE
 * tab's session. A session (DB row) stores the dictionary combo config
 * and default-folder data; several tabs MAY share the same session id -
 * each tab just creates its OWN independent WebSocket connection.
 * Do NOT call sendCreateSession here.
 */
const requestNewSession = (): void => {
    const sessionId = dictTabsStore.activeTab?.sessionId
    if (sessionId == null) {
        console.warn('[DictPage] No active tab session to clone.')
        return
    }
    dictTabsStore.createTab(sessionId)
}

/** Backend asked us to open a session: create (or activate) its tab. */
const handleCreateSession = (sessionId: number): void => {
    dictTabsStore.activateSessionTab(sessionId)
}

/** The tab's session died on the backend: close the tab. */
const handleSessionError = (tabId: string): void => {
    dictTabsStore.closeTab(tabId)
}

/** Window config says this tab should use another session: switch to it. */
const handleRedirectSession = (sessionId: number): void => {
    const tab = dictTabsStore.activeTab
    if (!tab) return
    const keyword = (route.query.keyword as string) || ''
    tab.initialKeyword = keyword
    tab.initialKeywordToShow = keyword
    tab.sessionId = sessionId
}

const handleActivateSession = (sessionId: number): void => {
    dictTabsStore.activateSessionTab(sessionId)
}

// --- Native context menu (right-click) ---
/**
 * The last text that was selected when the native menu was opened. The Rust
 * menu click only tells us WHICH item was chosen, so we keep the payload
 * here and apply it when the action event arrives.
 */
const lastContextMenuSelection = ref('')

/** Text of the current selection in the MAIN document ("" if none). */
const getMainDocumentSelection = (): string =>
    window.getSelection()?.toString().trim() ?? ''

/**
 * Open a NEW tab bound to the active session, then immediately look up
 * `text` in it (the tab deep-links via its per-tab initialKeyword once its
 * WebSocket opens). This is the "在 New Tab 中查询" menu action.
 */
const openTabWithKeyword = (text: string): void => {
    const sessionId = dictTabsStore.activeTab?.sessionId
    if (sessionId == null) {
        console.warn('[DictPage] No active tab session to clone.')
        return
    }
    const tabId = dictTabsStore.createTab(sessionId, text || 'New Tab', text, text)
    // dictTabsStore.activateTab(tabId)
}

/** Display name of a backend session (falls back to "Session <id>"). */
const sessionNameOf = (id: number): string =>
    dictTabsStore.activeController?.sessionsNameId.find((s) => s.id === id)?.name
    ?? `Session ${id}`

/** Open a brand-new main window, optionally bound to a session / keyword. */
const openNewWindow = (sessionId?: number, keyword?: string): void => {
    invoke(TAURI_CMD.OPEN_NEW_WINDOW, {
        sessionId: sessionId ?? null,
        keyword: keyword ?? '',
    }).catch((err) => console.error('[DictPage] open_new_window failed:', err))
}

/**
 * "Move Tab to New Window": spawn a new main window bound to the ACTIVE
 * tab's session (deep-linked with the current lookup keyword), then close
 * the tab in THIS window. Tab persistence flows automatically: the new
 * window seeds its first tab from the URL deep link (nothing persisted for
 * its fresh label yet), the closed tab is saved away by the store.
 */
const moveActiveTabToNewWindow = (): void => {
    const tab = dictTabsStore.activeTab
    if (tab?.sessionId == null) {
        console.warn('[DictPage] No active tab session to move.')
        return
    }
    const keyword = tab.controller?.lastSearchKeyword ?? tab.initialKeyword ?? ''
    openNewWindow(tab.sessionId, keyword)
    dictTabsStore.closeTab(tab.id)
}

/**
 * Dispatch a native context-menu action. Plain items map to strings;
 * session-submenu items carry their session id as a suffix, e.g.
 * "new-tab-session:3". Tab creation goes through the dictTabs store
 * (which persists the tab list per window), new-window actions ask Rust
 * to spawn another main window (which persists its own tabs by label).
 */
const handleContextMenuAction = (action: string): void => {
    if (action === 'new-tab') {
        requestNewSession()
        return
    }
    if (action === 'lookup-selection') {
        openTabWithKeyword(lastContextMenuSelection.value)
        return
    }
    if (action.startsWith('new-tab-session-lookup:')) {
        const id = Number(action.slice('new-tab-session-lookup:'.length))
        if (Number.isFinite(id)) {
            const sel = lastContextMenuSelection.value
            dictTabsStore.createTab(id, sessionNameOf(id), sel, sel)
        }
        return
    }
    if (action.startsWith('new-tab-session:')) {
        const id = Number(action.slice('new-tab-session:'.length))
        if (Number.isFinite(id)) dictTabsStore.createTab(id, sessionNameOf(id))
        return
    }
    if (action === 'move-tab-to-new-window') {
        moveActiveTabToNewWindow()
        return
    }
    if (action === 'new-window') {
        openNewWindow()
        return
    }
    if (action === 'new-window-lookup') {
        openNewWindow(undefined, lastContextMenuSelection.value)
        return
    }
    if (action.startsWith('new-window-session-lookup:')) {
        const id = Number(action.slice('new-window-session-lookup:'.length))
        if (Number.isFinite(id)) openNewWindow(id, lastContextMenuSelection.value)
        return
    }
    if (action.startsWith('new-window-session:')) {
        const id = Number(action.slice('new-window-session:'.length))
        if (Number.isFinite(id)) openNewWindow(id)
    }
}

/**
 * Ask Rust to pop up the NATIVE context menu at (x, y) CSS-pixel coords.
 * The selected text decides which items the menu shows (Rust builds it).
 */
const showContextMenu = (x: number, y: number, selectedText: string): void => {
    if (!isTauri()) return
    lastContextMenuSelection.value = selectedText
    const dpr = window.devicePixelRatio || 1
    // The active locale + session list let Rust build the native menu with
    // the SAME i18n strings and per-session submenus (see context_menu.rs).
    invoke(TAURI_CMD.SHOW_CONTEXT_MENU, {
        x: x * dpr,
        y: y * dpr,
        selectedText,
        locale: i18n.global.locale.value as string,
        sessions: dictTabsStore.activeController?.sessionsNameId ?? [],
        // Lets Rust decide whether to show "Move Tab to New Window"
        // (it is hidden when the window only has a single tab).
        tabCount: dictTabsStore.tabSize,
    }).catch((err) => console.error('[DictPage] show_context_menu failed:', err))
}

/** Right-click anywhere in the MAIN document. */
const onDocumentContextMenu = (e: MouseEvent): void => {
    if (!isTauri()) return // keep the browser's default menu in plain-web dev
    e.preventDefault() // suppress the webview's default browser menu
    showContextMenu(e.clientX, e.clientY, getMainDocumentSelection())
}

/**
 * Right-click inside a dictionary IFRAME. The iframe posts its selection
 * and click coords; coords were already converted to parent-window viewport
 * space by DictIframe, so we can pop the menu directly.
 */
const handleIframeContextMenu = (payload: {
    selectedText: string
    x: number
    y: number
}): void => {
    showContextMenu(payload.x, payload.y, payload.selectedText)
}

/** Keydown event from the iframe. */
const handleIframeKeydown = (evt: unknown): void => {
    dictTabsStore.iframeKeydownEvent = evt
}

// --- Tauri event listeners (target the ACTIVE tab only) ---
let unlistenTextSelected: (() => void) | null = null
let unlistenOcrResult: (() => void) | null = null
let unlistenCtxMenu: (() => void) | null = null

const setupTauriListeners = async (): Promise<void> => {
    // Idempotent: release previous subscriptions before rebinding, so
    // re-running on route change does not duplicate listeners.
    await unlistenTextSelected?.()
    unlistenTextSelected = null
    await unlistenOcrResult?.()
    unlistenOcrResult = null
    await unlistenCtxMenu?.()
    unlistenCtxMenu = null
    try {
        // Context-menu actions apply in every window that hosts a DictPage.
        // IMPORTANT: use getCurrentWindow().listen() (NOT the global listen)
        // for the context-menu action channel. The global `listen` defaults
        // to target { kind: 'Any' } and receives events addressed to ANY
        // window, so an emit_to() from Rust would still be delivered to
        // every main window (each one would then run handleContextMenuAction
        // and duplicate "Open New Window" on every launch). Window-scoped
        // listen + Rust's emit_to are the two halves of the fix.
        unlistenCtxMenu = await getCurrentWindow().listen(TAURI_EVENT.CTX_MENU_ACTION, (event) => {
            console.log('ctx_menu_action:', event.payload, "in window ", getCurrentWindow().label)
            handleContextMenuAction(event.payload as string)
        })

        if (envFromRoute.value === ENV.SELECTION) {
            unlistenTextSelected = await listen(TAURI_EVENT.TEXT_SELECTED, (event) => {
                dictTabsStore.redirectWord = event.payload as string
            })
        }

        if (envFromRoute.value === ENV.HELPER || envFromRoute.value === ENV.MAIN) {
            unlistenOcrResult = await listen(TAURI_EVENT.OCR_RESULT, (event) => {
                dictTabsStore.redirectWord = event.payload as string
            })
        }
    } catch (error) {
        console.error('Failed to bind Tauri event listeners:', error)
    }
}

// --- Window title follows the active tab's last lookup ---
watch(
    () => dictTabsStore.activeController?.lastSearchKeyword,
    async (val) => {
        const title = val || 'FstDict'
        document.title = title
        if (isTauri()) {
            try {
                await getCurrentWindow().setTitle(title)
            } catch (error) {
                console.error('Failed to set window title:', error)
            }
        }
    }
)


watch(() => tabsCount.value, (newVal) => {
    tabHeight.value = newVal > 1 ? TAB_HEIGHT : 0
})

watch(() => dictTabsStore.redirectWord,
    (newVal) => {
        if (dictTabsStore.activeController) {
            dictTabsStore.activeController.keyword = newVal
        }
    }
)

// --- Viewport / sidebar ---
const handleResize = (): void => {
    viewportWidth.value = window.innerWidth
}

const initHeaderPaddingRight = (): void => {
    if (!isTauri()) {
        headerPaddingRight.value = 0
        return
    }
    if (platform() === 'macos') {
        headerPaddingRight.value = 0
    } else if (envFromRoute.value === ENV.MAIN) {
        headerPaddingRight.value = 138
    }
}

// --- Lifecycle ---
/**
 * (Re)build the page for the route's session id. Runs on first mount AND on
 * in-page route changes (AppLayout routes to /dict/:newId without leaving the
 * page - the original single-session watcher on route.params.id).
 *
 * Unlike a full rebuild, a route change REBINDS ONLY the active tab's
 * WebSocket to the new session id; every other tab stays mounted and keeps
 * its connection and its state.
 */
const initDictPage = async (): Promise<void> => {
    envFromRoute.value = (route.query.env as string) || ''

    // Anki mode class on the body.
    if (envFromRoute.value === 'anki') {
        document.body.classList.add('anki-mode')
    } else {
        document.body.classList.remove('anki-mode')
    }

    document.title = 'FstDict'
    initHeaderPaddingRight()

    // Deep-link: a route keyword is looked up by the (rebound) active tab
    // right after its WebSocket opens.
    const keyword = (route.query.keyword as string) || ''
    const tab = dictTabsStore.activeTab
    if (tab && keyword) {
        tab.initialKeywordToShow = keyword
        tab.initialKeyword = keyword
    }
    if (isTauri()) {
        await setupTauriListeners()
    }
    showPopoverWordOptions.value = window.innerWidth < 700
}

onMounted(async () => {
    window.addEventListener('resize', handleResize)
    // Deep link from the native context menu ("Open New Window in the
    // Session"): seed the first tab with the requested session (used only
    // when nothing was persisted for this window label), then make sure a
    // tab bound to it is active so initDictPage's keyword deep link lands
    // on the right tab.
    const sessionFromQuery = Number(route.query.session as string) || null
    await dictTabsStore.loadTabs(sessionFromQuery)
    if (sessionFromQuery != null) dictTabsStore.activateSessionTab(sessionFromQuery)
    // Right-click anywhere in the main document -> native context menu.
    // document.addEventListener('contextmenu', onDocumentContextMenu)
    await initDictPage()
})

onUnmounted(async () => {
    await dictTabsStore.saveTabs()
    window.removeEventListener('resize', handleResize)
    document.removeEventListener('contextmenu', onDocumentContextMenu)
    unlistenTextSelected?.()
    unlistenOcrResult?.()
    unlistenCtxMenu?.()
    document.body.classList.remove('anki-mode')
})

watch(
    () => props.isMainSidebarCollapsed,
    (collapsed) => {
        if (!isTauri()) return
        if (props.showSidebar && platform() === 'macos' && envFromRoute.value === ENV.MAIN) {
            headerPaddingLeft.value = collapsed ? 100 : 0
        }
    }
)

watch(
    () => viewportWidth.value,
    (width) => {
        showPopoverWordOptions.value = width < 700
    }
)
</script>

<style scoped>
:deep(.no-padding-main) {
    padding: 0;
    flex: 1;
    overflow: hidden;
}
</style>
