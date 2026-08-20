<template>
    <div v-if="showError" class="error-suggestions">
        {{ errorMessage }}
    </div>
    <div v-if="showWarning" class="warn-suggestions">
        {{ warningMessage }}
    </div>
    <ThreeDotsLoader v-if="showLoading" class="loader-inline" />

    <UseVirtualList v-show="showResults" ref="listRef" :list="displayList"
        :options="{ itemHeight: ITEM_HEIGHT, overscan: 20 }" height="calc(100%)" class="list-container">
        <template #default="{ data, index }">
            <div class="item-content clickable-row" :class="{ 'is-selected': selectedWord === data }"
                :style="{ height: `${ITEM_HEIGHT}px` }" @click="handleWordClick(data)">
                <el-text class="truncated-text" :title="data">
                    {{ data }}
                </el-text>
            </div>
        </template>
    </UseVirtualList>
</template>

<script lang="ts" setup>
import { ref, computed, watch, nextTick } from 'vue'
import type { PropType } from 'vue'
import { UseVirtualList } from '@vueuse/components'

import ThreeDotsLoader from '@/components/Svgs/ThreeDotsLoader.vue'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import type { SessionConfig, WordInfoWithLastSearch } from '@/common/type-interface'
import { getDictSettingsForLookup } from '@/common/utility'

const ITEM_HEIGHT = 30

const props = defineProps({
    webSocket: {
        type: [SessionWebSocketService, null],
        required: true,
    },
    sessionConfig: {
        type: Object as PropType<SessionConfig>,
        required: true,
        default: () => ({}),
    },
    keyword: {
        type: String,
        required: true,
        default: '',
    },
    wordOptions: {
        type: Array,
        default: () => [],
    },
    searchHistory: {
        type: Array as PropType<WordInfoWithLastSearch[]>,
        required: true,
        default: () => [],
    },
})

const emit = defineEmits<{
    (e: 'select', word: string): void
}>()

// --- Refs ---
const listRef = ref<InstanceType<typeof UseVirtualList> | null>(null)
const selectedWord = ref<string | null>(null)

// --- State flags ---
const showHistory = computed(() => !props.keyword.trim())

const showError = computed(() =>
    props.wordOptions.length === 1 && props.wordOptions[0].startsWith('FSTD_ERROR')
)

const showWarning = computed(() =>
    props.wordOptions.length === 1 && props.wordOptions[0].startsWith('FSTD_WARN')
)

const showLoading = computed(() =>
    props.wordOptions.length === 1 && props.wordOptions[0].startsWith('FSTD_SEARCHING')
)

const showResults = computed(() => !showError.value && !showWarning.value && !showLoading.value)

const errorMessage = computed(() =>
    showError.value
        ? props.wordOptions[0].replace('FSTD_ERROR', '') || 'Unknown error'
        : ''
)

const warningMessage = computed(() =>
    showWarning.value
        ? props.wordOptions[0].replace('FSTD_WARN', '') || ''
        : ''
)

const displayList = computed(() => {
    if (showHistory.value) {
        return props.searchHistory.map((item) => item.word)
    }
    return props.wordOptions
})

// --- Actions ---
const handleWordClick = (word: string): void => {
    selectedWord.value = word
    props.webSocket?.sendLookupKeyword(
        word,
        props.sessionConfig.default_folder.id ?? null,
        getDictSettingsForLookup(props.sessionConfig.dict_setting_option_name),
        true
    )
}

// --- Watchers ---
watch(() => props.keyword, (val) => {
    selectedWord.value = val
})

watch(
    () => props.wordOptions,
    () => {
        nextTick(() => {
            const el = listRef.value?.$el as HTMLElement | undefined
            if (el) el.scrollTop = 0
        })
    },
    { deep: true }
)
</script>

<style scoped>
.loader-inline {
    margin-left: 1rem;
}

.list-container {
    border: 1px solid var(--el-border-color-light, #e4e7ed);
    border-radius: 4px;
}

.clickable-row {
    display: flex;
    align-items: center;
    padding: 0 16px;
    box-sizing: border-box;
    cursor: pointer;
    border-bottom: 1px solid var(--el-border-color-extra-light, #f2f6fc);
    transition: background-color 0.2s ease;
    min-width: 0;
}

.clickable-row:hover {
    background-color: var(--el-fill-color-light, #f5f7fa);
}

.clickable-row.is-selected {
    background-color: var(--el-color-primary-light-9, #ecf5ff);
}

.clickable-row.is-selected :deep(.el-text) {
    color: var(--el-color-primary, #409eff);
}

.truncated-text {
    display: block !important;
    width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
</style>