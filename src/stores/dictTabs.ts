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
import type {
    DictsInfo,
    SessionNameId,
    SessionConfig,
    DictsSettingInfo,
    FolderWords,
    WordInfoWithLastSearch,
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
    redirectWord: string

    // --- Results ---
    lookupResults: Record<string, string[]>
    noteContent: string
    leftHistory: boolean
    isWordFavorited: boolean
    /** Monotonic counter bumped on every lookup; views use it to reset scroll. */
    lookupSeq: number

    // --- Session config & meta ---
    sessionConfig: SessionConfig
    dictsInfo: DictsInfo
    sessionDictsSettingInfo: DictsSettingInfo
    sessionsNameId: SessionNameId[]
    refreshDicsSettingsInfoFlag: boolean

    // --- History / options ---
    folderWords: FolderWords
    searchHistory: WordInfoWithLastSearch[]
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
     * Immutable insertion index. Keeps the content-area v-for in stable order
     * (see tabsByInsertionOrder); the tab bar's own `tabs` array may be
     * reordered by drag, but this number never changes.
     */
    order: number
}

let tabIdSeed = 0
let tabOrderSeed = 0

/** Generate a unique, stable tab id. */
const nextTabId = (): string => `tab-${++tabIdSeed}`

export const useDictTabsStore = defineStore('dictTabs', {
    state: () => ({
        tabs: [] as DictTabEntry[],
        activeTabId: '' as string,
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
    },

    actions: {
        /**
         * Rebind the ACTIVE tab to a new session (in-page /dict/:id switch).
         * ONLY the active tab is affected - every other tab keeps its WebSocket
         * and its runtime state untouched. On first mount (no tabs yet) a fresh
         * initial tab is created instead.
         */
        resetActiveTabSession(sessionId: number | null): void {
            this.initialSessionId = sessionId
            if (this.tabs.length === 0) {
                this.createTab(sessionId)
                return
            }
            const tab = this.tabs.find((t) => t.id === this.activeTabId)
            if (tab) tab.sessionId = sessionId
        },

        /**
         * Create a new tab entry and activate it.
         * @param sessionId backend session to bind (null => idle tab, no WebSocket)
         * @param title tab label (usually updated to the searched word on first lookup)
         */
        createTab(sessionId: number | null = null, title = 'New Tab'): string {
            const id = nextTabId()
            this.tabs.push({ id, title, sessionId, controller: null, order: ++tabOrderSeed })
            this.activeTabId = id
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
        },

        /** Attach a tab's runtime controller (called by DictTabSession on mount). */
        registerController(tabId: string, controller: TabController): void {
            const tab = this.tabs.find((t) => t.id === tabId)
            if (tab) tab.controller = controller
        },

        /** Detach a tab's controller (called by DictTabSession on unmount). */
        unregisterController(tabId: string): void {
            const tab = this.tabs.find((t) => t.id === tabId)
            if (tab) tab.controller = null
        },

        /** Update a tab's label (used when a lookup gives the tab a real word). */
        setTabTitle(tabId: string, title: string): void {
            const tab = this.tabs.find((t) => t.id === tabId)
            if (tab) tab.title = title || 'New Tab'
        },
    },
})
