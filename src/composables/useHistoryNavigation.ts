import { ref, watch } from 'vue'
import type { Ref } from 'vue'
import type { WordInfoWithLastSearch} from '@/common/type-interface'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import { getDictSettingsForLookup } from '@/common/utility'
import type { SessionConfig } from '@/common/type-interface'

interface UseHistoryNavigationOptions {
    webSocket: Ref<SessionWebSocketService | null>
    sessionConfig: Ref<SessionConfig>
    searchHistory: Ref<WordInfoWithLastSearch[]>
    leftHistory: Ref<boolean>
    hasResultLastSearch: Ref<boolean>
}

export function useHistoryNavigation(options: UseHistoryNavigationOptions) {
    const { webSocket, sessionConfig, searchHistory, leftHistory, hasResultLastSearch } = options

    const historyIndex = ref(-1)
    const isHistoryTriggered = ref(false)
    const redirectHistoryWord = ref('')

    const handleHistoryBack = () => {
        if (historyIndex.value < searchHistory.value.length - 1) {
            historyIndex.value += 1
            isHistoryTriggered.value = true
            webSocket.value?.sendSearchHistoryRequest()
        }
    }

    const handleHistoryForward = () => {
        if (historyIndex.value > 0) {
            historyIndex.value -= 1
            isHistoryTriggered.value = true
            webSocket.value?.sendSearchHistoryRequest()
        }
    }

    const canGoBack = () => historyIndex.value < searchHistory.value.length - 1
    const canGoForward = () => historyIndex.value > 0 && historyIndex.value !== -1

    watch(leftHistory, (newVal) => {
        if (newVal) {
            isHistoryTriggered.value = false
            setTimeout(() => {
                historyIndex.value = hasResultLastSearch.value ? 0 : -1
            }, 100)
        }
    })

    watch(searchHistory, () => {
        if (isHistoryTriggered.value) {
            isHistoryTriggered.value = false
            const word = searchHistory.value[historyIndex.value]?.word
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
        historyIndex,
        redirectHistoryWord,
        handleHistoryBack,
        handleHistoryForward,
        canGoBack,
        canGoForward,
    }
}