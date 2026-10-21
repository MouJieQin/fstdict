import { ref, watch, computed } from 'vue'
import type { Ref } from 'vue'
import type { WordInfo } from '@/common/type-interface'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import { getDictSettingsForLookup } from '@/common/utility'
import type { SessionConfig } from '@/common/type-interface'
import { useDictTabsStore } from '@/stores'
const dictTabsStore = useDictTabsStore()

/** Runtime state of the ACTIVE tab (drives the shared TitleBar). */
const activeController = computed(() => dictTabsStore.activeController)

interface UseHistoryNavigationOptions {
    webSocket: Ref<SessionWebSocketService | null>
    sessionConfig: Ref<SessionConfig>
    searchHistory: Ref<WordInfo[]>
    leftHistory: Ref<boolean>
    hasResultLastSearch: Ref<boolean>
}

export function useHistoryNavigation(options: UseHistoryNavigationOptions) {
    const { webSocket, sessionConfig, searchHistory, leftHistory, hasResultLastSearch } = options

    // const historyIndex = ref(-1)
    const isHistoryTriggered = ref(false)
    const redirectHistoryWord = ref('')

    const handleHistoryBack = () => {
        if (activeController.value && activeController.value.historyIndex < searchHistory.value.length - 1) {
            activeController.value.historyIndex += 1
            isHistoryTriggered.value = true
            webSocket.value?.sendSearchHistoryRequest()
        }
    }

    const handleHistoryForward = () => {
        if (activeController.value && activeController.value.historyIndex > 0) {
            activeController.value.historyIndex -= 1
            isHistoryTriggered.value = true
            webSocket.value?.sendSearchHistoryRequest()
        }
    }

    const canGoBack = computed(() => activeController.value && activeController.value.historyIndex < searchHistory.value.length - 1)
    const canGoForward = computed(() => activeController.value && activeController.value.historyIndex > 0 && activeController.value.historyIndex !== -1)

    watch(leftHistory, (newVal) => {
        if (newVal) {
            isHistoryTriggered.value = false
            setTimeout(() => {
                if (activeController.value) {
                    activeController.value.historyIndex = hasResultLastSearch.value ? 0 : -1
                }
            }, 100)
        }
    })

    watch(searchHistory, () => {
        if (isHistoryTriggered.value) {
            isHistoryTriggered.value = false
            const word = activeController.value && searchHistory.value[activeController.value.historyIndex]?.word
            if (word) {
                redirectHistoryWord.value = word
                webSocket.value?.sendLookupKeyword(
                    word,
                    sessionConfig.value.default_folder.id,
                    getDictSettingsForLookup(sessionConfig.value.dict_setting_option_name),
                    false
                )
            }
        }
    })

    return {
        redirectHistoryWord,
        handleHistoryBack,
        handleHistoryForward,
        canGoBack,
        canGoForward,
    }
}