<template>
    <div class="floating-window-search-container" @mousedown="preventDrag = true" @mouseup="preventDrag = false">
        <!-- 1. Popover wrapper replaces the un-virtualized el-autocomplete slot framework -->
        <el-popover ref="popoverRef" trigger="contextmenu" placement="bottom-start" :visible="isDropdownVisible"
            :width="popoverWidth" :show-arrow="false" popper-class="virtual-autocomplete-popper">
            <template #reference>
                <!-- Standard input box wrapper -->
                <el-input ref="inputRef" v-model="keyword" placeholder="Search" clearable style="font-size: 1rem;"
                    @input="handleInputChange" @focus="handleFocus" @blur="handleBlur" @keyup.enter="handleEnter">
                    <!-- Prefix dropdown method picker remains untouched -->
                    <template #prefix>
                        <SearchMethodSelect
                            :searchMethod="props.sessionConfig.default_search_method?.method || 'prefix_search'"
                            @update-search-method="handleSearchMethodChange" />
                    </template>
                </el-input>
            </template>

            <!-- 2. Embedded Virtualized list container inside the active dropdown space -->
            <div class="virtual-dropdown-menu">
                <div v-if="links.length === 0" class="empty-suggestions">
                    No suggestions found
                </div>
                <UseVirtualList v-else ref="virtualListRef" :list="links" :options="{ itemHeight: 35, overscan: 10 }"
                    height="250px">
                    <template #default="{ data }">
                        <div class="suggestion-item" @mousedown.prevent="handleSelect(data)">
                            <span class="suggestion-text">{{ data.value }}</span>
                        </div>
                    </template>
                </UseVirtualList>
            </div>
        </el-popover>
    </div>
</template>

<script lang="ts" setup>
import { ref, watch, nextTick, onMounted } from 'vue'
import { UseVirtualList } from '@vueuse/components'
import { ElInput } from 'element-plus'
import { getDictSettingsForLookup } from '@/common/utility'
import SearchMethodSelect from '@/components/TitleBar/SearchMethodSelect.vue'


// Assuming interface matching your LinkItem mapping properties
interface LinkItem {
    value: string
    link: string
}

const props = defineProps<{
    webSocket: any
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

const inputRef = ref<InstanceType<typeof ElInput> | null>(null)
const popoverRef = ref<any>(null)
const virtualListRef = ref<any>(null)

let searchDebounceTimer: any = null

// Synchronizes the width of the dropdown to match the input box dynamically
onMounted(() => {
    if (inputRef.value?.$el) {
        popoverWidth.value = inputRef.value.$el.getBoundingClientRect().width
    }
})

// 3. Watch for changes from components or network history arrays to rebuild items safely
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
    // Instantly forces virtual tracking space back up to first element context
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
watch(() => props.redirectHisotryWord, (newVal) => {
    keyword.value = newVal
})

// 4. Refactored high-performance WebSocket debounce handler loop
const triggerAsyncSearch = () => {
    if (searchDebounceTimer) { clearTimeout(searchDebounceTimer) }

    // Use a standard 200ms debounce window to prevent flooding the WebSocket
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
    isDropdownVisible.value = true
    // triggerAsyncSearch() // Request fresh records instantly
}

const handleBlur = () => {
    // Delay slightly to allow row click click handler events to register first
    setTimeout(() => {
        isDropdownVisible.value = false
    }, 200)
}

const handleSelect = (item: LinkItem) => {
    keyword.value = item.value
    isDropdownVisible.value = false
    sendLookupKeyword()
}

const handleEnter = () => {
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
    // Re-run search query matching immediately using the newly selected method logic
    nextTick(() => triggerAsyncSearch())
}

const preventDrag = ref(false)
</script>

<style>
/* Global selector scope overrides to clean padding off popover targets */
.virtual-autocomplete-popper {
    padding: 0 !important;
    min-width: 0 !important;
    overflow: hidden;
    box-shadow: var(--el-box-shadow-light) !important;
}
</style>

<style scoped>
.virtual-dropdown-menu {
    background-color: var(--el-bg-color-overlay, #ffffff);
    border: 1px solid var(--el-border-color-light, #e4e7ed);
    border-radius: 4px;
    overflow: hidden;
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

.suggestion-item:hover {
    background-color: var(--el-fill-color-light, #f5f7fa);
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
