<template>
    <div class="floating-window-search-container">
        <el-input v-if="!showPopoverSuggestions" ref="inputRef" v-model="keyword" autocomplete="off" autocorrect="off"
            autocapitalize="off" spellcheck="false" placeholder="Search" clearable style="font-size: 1rem;"
            @input="handleInputChange" @focus="handleFocus" @blur="handleBlur" @keydown.down.prevent="handleKeyDown"
            @keydown.up.prevent="handleKeyUp" @keydown.enter.prevent="handleKeyEnter"
            @keydown.escape="isDropdownVisible = false">
            <template #prefix>
                <SearchMethodSelect :searchMethod="props.sessionConfig.default_search_method?.method || 'prefix_search'"
                    @update-search-method="handleSearchMethodChange" />
            </template>
        </el-input>
        <!-- 1. Restored clean popover bounds and synchronized width binding perfectly -->
        <el-popover v-else ref="popoverRef" trigger="contextmenu" placement="bottom-start" :visible="isDropdownVisible"
            :width="popoverWidth" :show-arrow="false" popper-class="virtual-autocomplete-popper" :teleported="true">
            <template #reference>
                <el-input ref="inputRef" v-model="keyword" autocomplete="off" autocorrect="off" autocapitalize="off"
                    spellcheck="false" placeholder="Search" clearable style="font-size: 1rem;"
                    @input="handleInputChange" @focus="handleFocus" @blur="handleBlur"
                    @keydown.down.prevent="handleKeyDown" @keydown.up.prevent="handleKeyUp"
                    @keydown.enter.prevent="handleKeyEnter" @keydown.escape="isDropdownVisible = false">
                    <template #prefix>
                        <SearchMethodSelect
                            :searchMethod="props.sessionConfig.default_search_method?.method || 'prefix_search'"
                            @update-search-method="handleSearchMethodChange" />
                    </template>
                </el-input>
            </template>

            <!-- 2. Embedded Virtualized list container -->
            <div class="virtual-dropdown-menu">
                <div v-if="links.length === 0" class="empty-suggestions">
                    No suggestions found
                </div>
                <div v-else-if="(links.length === 1 && links[0].value.startsWith('FSTD_ERROR'))"
                    class="error-suggestions">
                    {{ links[0].value.replace('FSTD_ERROR', '') || 'No error message' }}
                </div>
                <UseVirtualList v-show="(links.length >= 1 && (!links[0].value.startsWith('FSTD_ERROR')))"
                    ref="virtualListRef" :list="links" :options="{ itemHeight: 35, overscan: 10 }" height="250px">
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
import { ref, watch, computed, nextTick, onMounted, onBeforeUnmount } from 'vue'
import { UseVirtualList } from '@vueuse/components'
import { ElInput } from 'element-plus'
import { getDictSettingsForLookup } from '@/common/utility'
import SearchMethodSelect from '@/components/TitleBar/SearchMethodSelect.vue'

interface LinkItem {
    value: string
    link: string
}

const props = defineProps<{
    webSocket: any
    env: string
    sessionConfig: any
    redirectWord: string
    redirectHistoryWord: string
    searchHistory: Array<{ word: string }>
    wordOptions: string[]
}>()

const emits = defineEmits<{
    (e: 'change:keyword', keyword: string): void
}>()

const keyword = ref('')
const links = ref<LinkItem[]>([])
const isDropdownVisible = ref(false)
const popoverWidth = ref(300)
const activeIndex = ref(-1)

const inputRef = ref<InstanceType<typeof ElInput> | null>(null)
const popoverRef = ref<any>(null)
const virtualListRef = ref<any>(null)

let searchDebounceTimer: any = null
let resizeObserver: ResizeObserver | null = null

const showPopoverSuggestions = computed(() => {
    return props.env === 'anki'
})

// Setup layout trackers and global input listener bounds on initialization
onMounted(() => {
    window.addEventListener('keydown', handleGlobalKeydown)

    // Track panel layout adjustments to synchronize element outer border bounds exactly
    if (inputRef.value?.$el) {
        popoverWidth.value = inputRef.value.$el.offsetWidth

        resizeObserver = new ResizeObserver(() => {
            if (inputRef.value?.$el) {
                // Reads offsetWidth to include margins/borders, keeping popover size perfect
                popoverWidth.value = inputRef.value.$el.offsetWidth
            }
        })
        resizeObserver.observe(inputRef.value.$el)
    }
})

onBeforeUnmount(() => {
    window.removeEventListener('keydown', handleGlobalKeydown)
    if (resizeObserver) {
        resizeObserver.disconnect()
    }
})

// Automatically grabs focus if user begins raw typing while app layer is focused
const handleGlobalKeydown = (e: KeyboardEvent) => {
    const activeEl = document.activeElement
    if (
        activeEl &&
        (activeEl.tagName === 'INPUT' ||
            activeEl.tagName === 'TEXTAREA' ||
            (activeEl as HTMLElement).isContentEditable)
    ) {
        return
    }

    // Bypass application shortcut system handlers
    if (e.metaKey || e.ctrlKey || e.altKey || e.key === 'Escape' || e.key === 'Tab') {
        return
    }

    // If a valid plain string character is struck, move context focus to input
    if (e.key.length === 1 && inputRef.value) {
        keyword.value = ''
        inputRef.value.focus()
    }
}

const scrollToActiveItem = () => {
    if (!virtualListRef.value?.$el) return
    const container = virtualListRef.value.$el
    const itemHeight = 35
    const visibleHeight = 250
    const currentScrollTop = container.scrollTop
    const targetTopPosition = activeIndex.value * itemHeight

    if (targetTopPosition + itemHeight > currentScrollTop + visibleHeight) {
        container.scrollTop = targetTopPosition - visibleHeight + itemHeight
    } else if (targetTopPosition < currentScrollTop) {
        container.scrollTop = targetTopPosition
    }
}

const handleKeyDown = () => {
    if (!isDropdownVisible.value || links.value.length === 0) return
    if (activeIndex.value < links.value.length - 1) {
        activeIndex.value++
    } else {
        activeIndex.value = 0
    }
    scrollToActiveItem()
}

const handleKeyUp = () => {
    if (!isDropdownVisible.value || links.value.length === 0) return
    if (activeIndex.value > 0) {
        activeIndex.value--
    } else {
        activeIndex.value = links.value.length - 1
    }
    scrollToActiveItem()
}

const handleKeyEnter = () => {
    if (isDropdownVisible.value && activeIndex.value >= 0 && activeIndex.value < links.value.length) {
        handleSelect(links.value[activeIndex.value])
    } else {
        isDropdownVisible.value = false
        sendLookupKeyword()
    }
}

const syncSuggestions = () => {
    if (!keyword.value.trim()) {
        links.value = props.searchHistory.map(item => ({
            value: String(item.word),
            link: String(item.word),
        }))
    } else {
        links.value = props.wordOptions.map(item => ({
            value: String(item),
            link: String(item),
        }))
    }
    // activeIndex.value = links.value.length > 0 ? 0 : -1
    activeIndex.value = -1
    nextTick(() => {
        if (virtualListRef.value?.$el) virtualListRef.value.$el.scrollTop = 0
    })
}

watch(() => props.wordOptions, syncSuggestions, { deep: true })
watch(() => props.searchHistory, syncSuggestions, { deep: true })
watch(() => props.redirectWord, (newVal) => {
    keyword.value = newVal
    sendLookupKeyword()
})
watch(() => props.redirectHistoryWord, (newVal) => {
    keyword.value = newVal
})

const triggerAsyncSearch = () => {
    if (searchDebounceTimer) { clearTimeout(searchDebounceTimer) }
    searchDebounceTimer = setTimeout(() => {
        if (!keyword.value.trim()) {
            props.webSocket?.sendSearchHistoryRequest()
        } else {
            sendLookupKeyword(false)
            props.webSocket?.sendKeywordOptionsSearch(keyword.value, props.sessionConfig.default_search_method.method, getDictSettingsForLookup(props.sessionConfig.dictsSettingInfo || []))
        }
    }, 200)
}

const sendLookupKeyword = (leftHistory: boolean = true) => {
    props.webSocket?.sendLookupKeyword(keyword.value, props.sessionConfig.default_folder.id, getDictSettingsForLookup(props.sessionConfig.dictsSettingInfo || []), leftHistory)
}

const handleInputChange = () => {
    isDropdownVisible.value = true
    emits('change:keyword', keyword.value)
    triggerAsyncSearch()
}

const handleFocus = () => {
    if (!keyword.value.trim()) {
        links.value = props.searchHistory.map(item => ({
            value: String(item.word),
            link: String(item.word),
        }))
    }
    activeIndex.value = links.value.length > 0 ? 0 : -1
    isDropdownVisible.value = true
}

const handleBlur = () => {
    setTimeout(() => {
        isDropdownVisible.value = false
    }, 200)
}

const handleSelect = (item: LinkItem) => {
    keyword.value = item.value
    isDropdownVisible.value = false
    sendLookupKeyword()
}

const handleSearchMethodChange = (newMethod: string) => {
    if (props.sessionConfig.default_search_method) {
        props.sessionConfig.default_search_method.method = newMethod
    } else {
        props.sessionConfig.default_search_method = { method: newMethod }
    }
    props.webSocket?.sendSessionConfig(props.sessionConfig)
    nextTick(() => triggerAsyncSearch())
}
</script>

<style>
/* Scoped overrides to eliminate unpredictable popover borders and paddings */
.virtual-autocomplete-popper {
    padding: 0 !important;
    min-width: 0 !important;
    overflow: hidden;
    box-shadow: var(--el-box-shadow-light) !important;
    border: 1px solid var(--el-border-color-light, #e4e7ed) !important;
    background-color: var(--el-bg-color-overlay, #ffffff) !important;
}
</style>

<style scoped>
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

.error-suggestions {
    padding: 16px;
    text-align: center;
    color: var(--el-text-color-danger, #f56c6c);
    font-size: 13px;
}
</style>
