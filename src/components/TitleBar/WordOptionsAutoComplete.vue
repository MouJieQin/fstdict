<template>
    <div class="floating-window-search-container">
        <el-input v-if="!showPopoverWordOptions" ref="inputRef" v-model="keyword" autocomplete="off" autocorrect="off"
            autocapitalize="off" spellcheck="false" :placeholder="$t('common.search')" clearable class="search-input"
            @input="onInputChange" @keydown.enter.prevent="onKeyEnter" @compositionstart="onCompositionStart"
            @focus="handleFocus(emit)" @blur="handleBlur(emit)" @compositionend="onCompositionEnd">
            <template #prefix>
                <SearchMethodSelect :search-method="sessionConfig.default_search_method?.method || 'prefix_search'"
                    @update-search-method="handleSearchMethodChange" />
            </template>
        </el-input>

        <el-popover v-else trigger="contextmenu" placement="bottom-start" :visible="isDropdownVisible"
            :width="popoverWidth" :show-arrow="false" popper-class="virtual-autocomplete-popper" :teleported="true">
            <template #reference>
                <el-input ref="inputRef" v-model="keyword" autocomplete="off" autocorrect="off" autocapitalize="off"
                    spellcheck="false" :placeholder="$t('common.search')" clearable class="search-input"
                    @input="onInputChange" @focus="handleFocusWithPopover(emit)" @keydown.down.prevent="handleKeyDown"
                    @keydown.up.prevent="handleKeyUp" @keydown.enter.prevent="onKeyEnter"
                    @blur="handleBlurWithPopover(emit)" @keydown.escape="isDropdownVisible = false"
                    @compositionstart="onCompositionStart" @compositionend="onCompositionEnd">
                    <template #prefix>
                        <SearchMethodSelect
                            :search-method="sessionConfig.default_search_method?.method || 'prefix_search'"
                            @update-search-method="handleSearchMethodChange" />
                    </template>
                </el-input>
            </template>

            <div class="virtual-dropdown-menu" @mousedown.prevent>
                <div v-if="isEmptyState" class="empty-suggestions">
                    {{ $t('common.noSuggestions') }}
                </div>
                <div v-else-if="isErrorState" class="error-suggestions">
                    {{ errorMessage }}
                </div>
                <div v-else-if="isWarnState" class="warn-suggestions">
                    {{ warningMessage }}
                </div>
                <ThreeDotsLoader v-else-if="isSearchingState" class="loader-inline" />

                <UseVirtualList v-show="isResultState" ref="virtualListRef" :list="links"
                    :options="{ itemHeight: AUTOCOMPLETE_ITEM_HEIGHT, overscan: 10 }" height="250px">
                    <template #default="{ data, index }">
                        <div class="suggestion-item" :class="{ 'is-active': index === activeIndex }"
                            @mousedown.prevent="handleSelect(data)" @mouseenter="activeIndex = index">
                            <span class="suggestion-text">{{ data.value }}</span>
                        </div>
                    </template>
                </UseVirtualList>
            </div>
        </el-popover>
    </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { UseVirtualList } from '@vueuse/components'
import type { ElInput } from 'element-plus'
import SearchMethodSelect from '@/components/TitleBar/SearchMethodSelect.vue'
import ThreeDotsLoader from '@/components/Svgs/ThreeDotsLoader.vue'
import { useAutocomplete } from '@/composables/useAutocomplete'
import type { SessionWebSocketService } from '@/common/session-websocket-client'
import type { SessionConfig, WordInfoWithLastSearch } from '@/common/type-interface'
import { useI18n } from 'vue-i18n'


const props = defineProps<{
    webSocket: SessionWebSocketService | null
    env: string
    sessionConfig: SessionConfig
    redirectWord: string
    redirectHistoryWord: string
    searchHistory: WordInfoWithLastSearch[]
    wordOptions: string[]
    showPopoverWordOptions: boolean
    focusInputFlag: boolean
    firstChar: string,
    firstKeyCode: string
}>()

const emit = defineEmits<{
    (e: 'change:keyword', keyword: string): void
    (e: 'change:inputFocus', focus: boolean): void
}>()

const inputRef = ref<InstanceType<typeof ElInput> | null>(null)
const virtualListRef = ref<unknown>(null)
const popoverWidth = ref(300)
const isInputFocused = ref(false)
// State to track if we are waiting for focus to settle
const awaitingImeTrigger = ref(false);

let resizeObserver: ResizeObserver | null = null
let rafId: number | null = null

// Reactive refs for composable
const wsRef = computed(() => props.webSocket)
const configRef = computed(() => props.sessionConfig)
const historyRef = computed(() => props.searchHistory)
const optionsRef = computed(() => props.wordOptions)
const showPopoverRef = computed(() => props.showPopoverWordOptions)
const firstCharRef = computed(() => props.firstChar)
const firstKeyCodeRef = computed(() => props.firstKeyCode)

const { t } = useI18n()

const {
    keyword,
    links,
    isDropdownVisible,
    activeIndex,
    isComposing,
    handleKeyDown,
    handleKeyUp,
    handleKeyEnter: onKeyEnter,
    handleInputChange,
    handleFocus,
    handleFocusWithPopover,
    handleBlur,
    handleBlurWithPopover,
    sendLookupKeyword,
    sendKeywordOptionsSearch,
    AUTOCOMPLETE_ITEM_HEIGHT,
} = useAutocomplete({
    webSocket: wsRef,
    sessionConfig: configRef,
    searchHistory: historyRef,
    wordOptions: optionsRef,
    showPopover: showPopoverRef,
    firstChar: firstCharRef,
    firstKeyCode: firstKeyCodeRef,
    isInputFocused: isInputFocused,
    awaitingImeTrigger: awaitingImeTrigger
})

// --- Computed state flags for template clarity ---
const isEmptyState = computed(() => links.value.length === 0)

const isErrorState = computed(() =>
    links.value.length === 1 && links.value[0].value.startsWith('FSTD_ERROR')
)

const isWarnState = computed(() =>
    links.value.length === 1 && links.value[0].value.startsWith('FSTD_WARN')
)

const isSearchingState = computed(() =>
    links.value.length === 1 && links.value[0].value.startsWith('FSTD_SEARCHING')
)

const isResultState = computed(() =>
    links.value.length >= 1 &&
    !links.value[0].value.startsWith('FSTD_ERROR') &&
    !links.value[0].value.startsWith('FSTD_WARN') &&
    !links.value[0].value.startsWith('FSTD_SEARCHING')
)

const errorMessage = computed(() =>
    isErrorState.value
        ? links.value[0].value.replace('FSTD_ERROR', '') || 'Unknown error'
        : ''
)

const warningMessage = computed(() =>
    isWarnState.value
        ? links.value[0].value.replace('FSTD_WARN', '') || ''
        : ''
)

// --- Event handlers ---
const onInputChange = () => handleInputChange(emit)

const onCompositionStart = () => {
    isComposing.value = true
}

const onCompositionEnd = () => {
    // Small delay to ensure IME commit completes before processing
    setTimeout(() => {
        isComposing.value = false
    }, 20)
}

const handleSelect = (item: { value: string; link: string }) => {
    keyword.value = item.value
    isDropdownVisible.value = false
    sendLookupKeyword()
}

const handleSearchMethodChange = (newMethod: string) => {
    const config = props.sessionConfig
    if (config.default_search_method) {
        config.default_search_method.method = newMethod
    } else {
        ; (config as SessionConfig).default_search_method = { method: newMethod }
    }
    props.webSocket?.sendSessionConfig(config)
    nextTick(() => handleInputChange(emit))
}

const scrollToActiveItem = () => {
    const el = virtualListRef.value as { $el?: HTMLElement } | null
    if (!el?.$el) return

    const container = el.$el
    const visibleHeight = 250
    const targetTop = activeIndex.value * AUTOCOMPLETE_ITEM_HEIGHT

    if (targetTop + AUTOCOMPLETE_ITEM_HEIGHT > container.scrollTop + visibleHeight) {
        container.scrollTop = targetTop - visibleHeight + AUTOCOMPLETE_ITEM_HEIGHT
    } else if (targetTop < container.scrollTop) {
        container.scrollTop = targetTop
    }
}

const focusAndClearInput = () => {
    if (isInputFocused.value) return
    inputRef.value?.clear()
    awaitingImeTrigger.value = true;
    inputRef.value?.focus();
}

watch(() => props.focusInputFlag, () => {
    focusAndClearInput()
})

// Sync scroll when active index changes
watch(activeIndex, scrollToActiveItem)

// Sync suggestions when props change
watch(optionsRef, () => {
    // Handled internally by composable
    nextTick(() => {
        const el = virtualListRef.value as { $el?: HTMLElement } | null
        if (el?.$el) el.$el.scrollTop = 0
    })
}, { deep: true })

// Handle redirect word from parent
watch(() => props.redirectWord, (newVal) => {
    keyword.value = newVal
    // Trigger lookup and option search
    sendLookupKeyword()
    sendKeywordOptionsSearch()
})

watch(() => props.redirectHistoryWord, (newVal) => {
    keyword.value = newVal
})

const resizePopoverObserver = (el: HTMLElement | null) => {
    // clear previous
    if (rafId) cancelAnimationFrame(rafId)
    resizeObserver?.disconnect()

    if (!el) return

    resizeObserver = new ResizeObserver((entries) => {
        const entry = entries[0]
        if (!entry) return
        const w = entry.contentRect.width

        rafId = requestAnimationFrame(() => {
            popoverWidth.value = w
        })
    })

    resizeObserver.observe(el)
    popoverWidth.value = el.getBoundingClientRect().width
}


// --- Resize observer for popover width ---
watch(
    () => inputRef.value?.$el as HTMLElement | null,
    (el) => {
        resizePopoverObserver(el)

        emit('change:inputFocus', false)
        isInputFocused.value = false
    },
    { immediate: true }
)

onMounted(() => {
})

onBeforeUnmount(() => {
    if (rafId) cancelAnimationFrame(rafId)
    resizeObserver?.disconnect()
    resizeObserver = null
})
</script>

<style scoped>
.loader-inline {
    margin-left: 1rem;
}

.virtual-dropdown-menu {
    background-color: var(--el-bg-color-overlay, #ffffff);
    overflow: hidden;
    border-radius: 4px;
}

.suggestion-item {
    display: flex;
    align-items: center;
    height: 35px;
    padding: 0 12px;
    box-sizing: border-box;
    cursor: pointer;
    transition: background-color 0.15s ease;
}

.suggestion-item:hover,
.suggestion-item.is-active {
    background-color: var(--el-fill-color-light, #f5f7fa);
}

.suggestion-item.is-active .suggestion-text {
    color: var(--el-color-primary, #409eff);
    font-weight: 500;
}

.suggestion-text {
    font-size: 14px;
    color: var(--el-text-color-regular, #606266);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
}

.empty-suggestions {
    padding: 16px;
    text-align: center;
    color: var(--el-text-color-secondary, #909399);
    font-size: 13px;
}
</style>

<style>
/* Global popover override - must be unscoped */
.virtual-autocomplete-popper {
    padding: 0 !important;
    min-width: 0 !important;
    overflow: hidden;
    box-shadow: var(--el-box-shadow-light) !important;
    border: 1px solid var(--el-border-color-light, #e4e7ed) !important;
    background-color: var(--el-bg-color-overlay, #ffffff) !important;
}
</style>