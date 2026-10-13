/**
 * dictTabs.ts
 * ------------------------------------------------------------------
 * Pinia store for the multi-tab dictionary page.
 *
 * Responsibilities are split between the store and the per-tab session
 * component so that "one tab == one independent DictPage":
 *
 *   store       = tab list bookkeeping only (which tabs exist, their order,
 *                 session ids, and who is active)
 *   controller  = a reactive object OWNED by DictTabSession that holds ALL
 *                 of one tab's live runtime state (its own WebSocket,
 *                 lookup results, session config, history, ...)
 *
 * The shared TitleBar never talks to the store's tabs directly - it reads
 * the ACTIVE tab's controller via the activeController getter, so every
 * prop automatically follows tab switches.
 */

import { defineStore } from 'pinia'
import { Store } from '@tauri-apps/plugin-store'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { isTauri } from '@tauri-apps/api/core'
import type {
    DictsInfo,
    SessionNameId,
    SessionConfig,
    DictsSettingInfo,
    FolderWords,
    WordInfo,
} from '@/common/type-interface'
import type { SessionWebSocketService } from '@/common/session-websocket-client'

/**
 * Reactive runtime state of ONE dictionary tab.
 * Created by DictTabSession.vue (a single `reactive()` object with plain
 * fields - no refs inside), registered into the store on mount and
 * unregistered on unmount.
 */
export interface TabController {
    // --- Connection ---
    webSocket: SessionWebSocketService | null
    sessionId: number | null

    // --- Input / lookup ---
    keyword: string
    lastSearchKeyword: string
    hasResultLastSearch: boolean
    // redirectWord: string

    // --- Results ---
    lookupResults: Record<string, string[]>
    noteContent: string
    leftHistory: boolean
    isWordFavorited: boolean
    /** Monotonic counter bumped on every lookup; views use it to reset scroll. */
    lookupSeq: number

    // --- Session config & meta ---
    showConfigPanel: boolean
    sessionConfig: SessionConfig
    dictsInfo: DictsInfo
    sessionDictsSettingInfo: DictsSettingInfo
    sessionsNameId: SessionNameId[]
    refreshDicsSettingsInfoFlag: boolean

    // --- History / options ---
    folderWords: FolderWords
    searchHistory: WordInfo[]
    wordOptions: string[]

    // --- Misc ---
    iframeKeydownEvent: unknown
    ankiProgress: Record<string, any>
    addDictMsgs: any[]
}

/** A single tab in the tab bar (metadata only - runtime state lives in controller). */
export interface DictTabEntry {
    id: string
    title: string
    sessionId: number | null
    controller: TabController | null
    /**
     * Keyword this tab should look up immediately after its WebSocket opens
     * (deep link). Usually ""; set when a tab is created from a context-menu
     * "lookup selection" action or from the route query.
     */
    initialKeyword: string

    initialKeywordToShow: string
    /**
     * Immutable insertion index. Keeps the content-area v-for in stable order
     * (see tabsByInsertionOrder); the tab bar's own `tabs` array may be
     * reordered by drag, but this number never changes.
     */
    order: number
}

export interface DictTabEntryState {
    id: string
    title: string
    sessionId: number | null
    initialKeyword: string
    initialKeywordToShow: string
}

export interface DictTabsState {
    tabs: DictTabEntryState[]
    activeTabId: string
    initialSessionId: number | null
}

let tabIdSeed = 0
let tabOrderSeed = 0

/** Generate a unique, stable tab id. */
const nextTabId = (): string => `tab-${++tabIdSeed}`

export const useDictTabsStore = defineStore('dictTabs', {
    state: () => ({
        tabs: [] as DictTabEntry[],
        activeTabId: '' as string,
        redirectWord: '' as string,
        /** Session id used to re-open a fresh tab when the last one is closed. */
        initialSessionId: null as number | null,
    }),

    getters: {
        /** The currently selected tab entry. */
        activeTab(state): DictTabEntry | undefined {
            return state.tabs.find((t) => t.id === state.activeTabId)
        },
        /** Runtime state of the currently selected tab (null before its session mounts). */
        activeController(state): TabController | null {
            return state.tabs.find((t) => t.id === state.activeTabId)?.controller ?? null
        },
        /**
         * Tab list in IMMUTABLE insertion order. The content area renders from
         * this instead of `tabs`, so drag reorders of the tab bar never cause
         * Vue to move DictTabSession DOM nodes (which would reload the dictionary
         * iframes and blank them out).
         */
        tabsByInsertionOrder(state): DictTabEntry[] {
            return [...state.tabs].sort((a, b) => a.order - b.order)
        },

        /** Convert the store state to a serializable format. */
        toSerializableState(state): DictTabsState {
            return {
                tabs: state.tabs.map((t) => ({
                    id: t.id,
                    title: t.title,
                    initialKeyword: t.controller?.lastSearchKeyword ?? '',
                    initialKeywordToShow: t.controller?.keyword ?? '',
                    sessionId: t.sessionId,
                })),
                activeTabId: state.activeTabId,
                initialSessionId: state.initialSessionId,
            }
        },
    },

    actions: {
        /** Save the current tab list to disk. */
        async saveTabs(): Promise<void> {
            if (!isTauri()) return
            const windowLabel = getCurrentWindow().label
            const store = await Store.load(`windows/dict-tabs.json`)
            await store.set(`${windowLabel}`, this.toSerializableState)
        },

        async loadTabs(): Promise<void> {
            if (!isTauri()) return
            const windowLabel = getCurrentWindow().label
            const store = await Store.load(`windows/dict-tabs.json`)
            const windowTabs = await store.get(`${windowLabel}`) as DictTabsState
            if (!windowTabs) {
                this.initialSessionId = 1
                this.createTab(this.initialSessionId)
                return
            }
            let activeTabId = ''
            for (const tab of windowTabs.tabs) {
                const tabId = this.createTab(tab.sessionId, tab.title, tab.initialKeywordToShow, tab.initialKeyword, false)
                if (tab.id === windowTabs.activeTabId) activeTabId = tabId
            }
            this.activeTabId = activeTabId
        },

        /**
         * Create a new tab entry and activate it.
         * @param sessionId backend session to bind (null => idle tab, no WebSocket)
         * @param title tab label (usually updated to the searched word on first lookup)
         * @param initialKeyword keyword to look up once this tab's WebSocket opens
         */
        createTab(
            sessionId: number | null = null,
            title = 'New Tab',
            initialKeywordToShow = '',
            initialKeyword: string = initialKeywordToShow,
            save = true,
        ): string {
            const id = nextTabId()
            this.tabs.push({
                id,
                title,
                sessionId,
                controller: null,
                initialKeyword,
                initialKeywordToShow,
                order: ++tabOrderSeed,
            })
            this.activeTabId = id
            if (save) this.saveTabs()
            return id
        },

        /** Create (or activate an existing) tab bound to a backend session. */
        activateSessionTab(sessionId: number): string {
            const existing = this.tabs.find((t) => t.sessionId === sessionId)
            if (existing) {
                this.activeTabId = existing.id
                return existing.id
            }
            return this.createTab(sessionId)
        },

        /** Activate an existing tab by id (no-op when the id is unknown). */
        activateTab(id: string): void {
            if (this.tabs.some((t) => t.id === id)) this.activeTabId = id
            this.saveTabs()
        },

        /**
         * Close a tab. Always keeps at least one tab alive; when the very last
         * tab is closed a fresh one is re-opened on the initial session id.
         */
        closeTab(id: string): void {
            const index = this.tabs.findIndex((t) => t.id === id)
            if (index === -1) return

            const wasActive = this.activeTabId === id
            this.tabs.splice(index, 1)

            if (this.tabs.length === 0) {
                this.createTab(this.initialSessionId)
                return
            }

            if (wasActive) {
                // Prefer the tab that slid into the same position, else the previous one.
                const next = this.tabs[index] ?? this.tabs[index - 1]
                this.activeTabId = next.id
            }
            this.saveTabs()
        },

        /**
         * Reorder tabs after a drag & drop.
         * A brand-new array is assigned so Vue's keyed v-for reliably re-renders.
         */
        reorderTabs(oldIndex: number, newIndex: number): void {
            if (oldIndex === newIndex) return
            const tabs = [...this.tabs]
            const [moved] = tabs.splice(oldIndex, 1)
            tabs.splice(newIndex, 0, moved)
            this.tabs = tabs
            this.saveTabs()
        },

        /** Attach a tab's runtime controller (called by DictTabSession on mount). */
        registerController(tabId: string, controller: TabController): void {
            const tab = this.tabs.find((t) => t.id === tabId)
            if (tab) tab.controller = controller
            this.saveTabs()
        },

        /** Detach a tab's controller (called by DictTabSession on unmount). */
        unregisterController(tabId: string): void {
            const tab = this.tabs.find((t) => t.id === tabId)
            if (tab) tab.controller = null
            this.saveTabs()
        },

        /** Update a tab's label (used when a lookup gives the tab a real word). */
        setTabTitle(tabId: string, title: string): void {
            const tab = this.tabs.find((t) => t.id === tabId)
            if (tab) tab.title = title || 'New Tab'
            this.saveTabs()
        },
    },
})
