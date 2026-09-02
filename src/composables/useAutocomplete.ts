import { ref, watch, nextTick } from 'vue'
import type { Ref } from 'vue'
import type { ElInput } from 'element-plus'
import { willScanAllFstNodes, getDictSettingsForLookup } from '@/common/utility'
import type { SessionWebSocketService } from '@/common/session-websocket-client'
import type { SessionConfig, WordInfoWithLastSearch } from '@/common/type-interface'
import {
    DEBOUNCE_SEARCH_MS,
    AUTOCOMPLETE_ITEM_HEIGHT,
    OPTION_PREFIX,
    SEARCH_METHOD,
} from '@/common/constants'
import { useI18n } from 'vue-i18n'

interface LinkItem {
    value: string
    link: string
}

interface UseAutocompleteOptions {
    webSocket: Ref<SessionWebSocketService | null>
    sessionConfig: Ref<SessionConfig>
    searchHistory: Ref<WordInfoWithLastSearch[]>
    wordOptions: Ref<string[]>
    showPopover: Ref<boolean>
    firstChar: Ref<string>
    firstKeyCode: Ref<string>
    isInputFocused: Ref<boolean>
    awaitingImeTrigger: Ref<boolean>
}

export function useAutocomplete(options: UseAutocompleteOptions) {
    const { webSocket, sessionConfig, searchHistory, wordOptions, showPopover, firstChar, firstKeyCode, isInputFocused, awaitingImeTrigger } = options
    const { t } = useI18n()

    const keyword = ref('')
    const links = ref<LinkItem[]>([])
    const isDropdownVisible = ref(false)
    const activeIndex = ref(-1)
    const isComposing = ref(false)
    const optionsReceivedFlag = ref(true)
    const lastKeywordForOptionSearch = ref('')

    let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null

    const syncSuggestions = () => {
        if (!keyword.value.trim()) {
            links.value = searchHistory.value.map(item => ({
                value: String(item.word),
                link: String(item.word),
            }))
        } else {
            links.value = wordOptions.value.map(item => ({
                value: String(item),
                link: String(item),
            }))
        }
        activeIndex.value = -1
        nextTick(() => {
            // Scroll reset handled by parent ref
        })
    }

    const sendKeywordOptionsSearch = (forced = false) => {
        lastKeywordForOptionSearch.value = keyword.value
        const method = sessionConfig.value.default_search_method.method

        if (method === SEARCH_METHOD.REGEX) {
            if (!keyword.value.trim()) return

            if (willScanAllFstNodes(keyword.value)) {
                if (!forced) {
                    const warning = `${OPTION_PREFIX.WARN}${t('autocomplete.regexPerformanceWarn', { pattern: keyword.value })}`
                    webSocket.value?.sendKeywordOptionsNote(keyword.value, warning)
                    return
                }
            }
        }

        webSocket.value?.sendKeywordOptionsNote(keyword.value, `${OPTION_PREFIX.SEARCHING}${t('autocomplete.searching')}`)
        webSocket.value?.sendKeywordOptionsSearch(
            keyword.value,
            method,
            getDictSettingsForLookup(sessionConfig.value.dict_setting_option_name)
        )
    }

    const sendLookupKeyword = (leftHistory = true) => {
        const method = sessionConfig.value.default_search_method.method
        if (method === SEARCH_METHOD.REGEX && willScanAllFstNodes(keyword.value)) {
            return
        }

        webSocket.value?.sendLookupKeyword(
            keyword.value.trim(),
            sessionConfig.value.default_folder.id,
            getDictSettingsForLookup(sessionConfig.value.dict_setting_option_name),
            leftHistory
        )
    }

    const triggerAsyncSearch = () => {
        if (searchDebounceTimer) clearTimeout(searchDebounceTimer)

        searchDebounceTimer = setTimeout(() => {
            if (!keyword.value.trim()) {
                webSocket.value?.sendSearchHistoryRequest()
            } else {
                sendLookupKeyword(false)
                if (optionsReceivedFlag.value) {
                    optionsReceivedFlag.value = false
                    sendKeywordOptionsSearch()
                }
            }
        }, DEBOUNCE_SEARCH_MS)
    }

    const handleKeyDown = () => {
        if (!isDropdownVisible.value || links.value.length === 0) return
        activeIndex.value = activeIndex.value < links.value.length - 1
            ? activeIndex.value + 1
            : 0
    }

    const handleKeyUp = () => {
        if (!isDropdownVisible.value || links.value.length === 0) return
        activeIndex.value = activeIndex.value > 0
            ? activeIndex.value - 1
            : links.value.length - 1
    }

    const handleKeyEnter = () => {
        if (isComposing.value) return

        const method = sessionConfig.value.default_search_method.method
        if (method === SEARCH_METHOD.REGEX) {
            if (keyword.value.trim() && willScanAllFstNodes(keyword.value)) {
                if (optionsReceivedFlag.value) {
                    optionsReceivedFlag.value = false
                    sendKeywordOptionsSearch(true)
                }
            }
        }

        if (!showPopover.value) {
            sendLookupKeyword()
        } else {
            if (isDropdownVisible.value && activeIndex.value >= 0 && activeIndex.value < links.value.length) {
                const selected = links.value[activeIndex.value]
                keyword.value = selected.value
                isDropdownVisible.value = false
                sendLookupKeyword()
            } else {
                isDropdownVisible.value = false
                sendLookupKeyword()
            }
        }
    }

    // The Focus Handler (Bind this to your el-input or input)
    const handleInputFocus = async (emit: (e: 'change:inputFocus', v: boolean) => void) => {
        isInputFocused.value = true
        emit('change:inputFocus', true)
        if (awaitingImeTrigger.value) {
            // Reset flag immediately
            awaitingImeTrigger.value = false;

            // Wait one tick for the browser renderer to paint the cursor
            await nextTick();

            // Now invoke Rust - the input is guaranteed to be the active element
            if (firstChar.value && firstChar.value.length === 1) {
                if (firstChar.value[0] >= 'a' && firstChar.value[0] <= 'z' || firstChar.value[0] >= 'A' && firstChar.value[0] <= 'Z') {
                    webSocket.value?.sendSimulateKeyPress(firstKeyCode.value);
                } else {
                    keyword.value = firstChar.value;
                }
            }
        }
    }

    const handleInputBlur = (emit: (e: 'change:inputFocus', v: boolean) => void) => {
        emit('change:inputFocus', false)
        isInputFocused.value = false
    }


    const handleInputChange = (emit: (e: 'change:keyword', v: string) => void) => {
        if (showPopover.value) {
            isDropdownVisible.value = true
        }
        emit('change:keyword', keyword.value)
        triggerAsyncSearch()
    }

    const handleFocus = async (emit: (e: 'change:inputFocus', v: boolean) => void) => {
        if (!keyword.value.trim()) {
            webSocket.value?.sendSearchHistoryRequest()
        }
        await handleInputFocus(emit)
    }

    const handleFocusWithPopover = async (emit: (e: 'change:inputFocus', v: boolean) => void) => {
        if (!keyword.value.trim()) {
            links.value = searchHistory.value.map(item => ({
                value: String(item.word),
                link: String(item.word),
            }))
        }
        activeIndex.value = links.value.length > 0 ? 0 : -1
        isDropdownVisible.value = true

        await handleInputFocus(emit)
    }

    const handleBlur = (emit: (e: 'change:inputFocus', v: boolean) => void) => {
        handleInputBlur(emit)
    }

    const handleBlurWithPopover = (emit: (e: 'change:inputFocus', v: boolean) => void) => {
        isDropdownVisible.value = false
        handleInputBlur(emit)
    }


    // Watchers
    watch(wordOptions, () => {
        const isSearching = wordOptions.value.length === 1 && wordOptions.value[0].startsWith(OPTION_PREFIX.SEARCHING)
        if (!isSearching) {
            optionsReceivedFlag.value = true
        }
        syncSuggestions()
    }, { deep: true })

    watch(searchHistory, syncSuggestions, { deep: true })

    watch(optionsReceivedFlag, (newVal) => {
        if (newVal && lastKeywordForOptionSearch.value !== keyword.value) {
            sendKeywordOptionsSearch()
        }
    })

    return {
        keyword,
        links,
        isDropdownVisible,
        activeIndex,
        isComposing,
        handleKeyDown,
        handleKeyUp,
        handleKeyEnter,
        handleInputChange,
        handleFocus,
        handleFocusWithPopover,
        handleBlur,
        handleBlurWithPopover,
        sendLookupKeyword,
        sendKeywordOptionsSearch,
        AUTOCOMPLETE_ITEM_HEIGHT,
    }
}