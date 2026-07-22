<template>
    <div v-if="showErrorSuggestion" class="error-suggestions">
        {{ props.wordOptions[0].replace('FSTD_ERROR', '') || 'No error message' }}
    </div>
    <div v-if="showWarnSuggestion" class="warn-suggestions">
        {{ props.wordOptions[0].replace('FSTD_WARN', '') || 'No warn message' }}
    </div>
    <ThreeDotsLoader v-if="showSearchIcon" style="margin-left:1rem;" />
    <!-- showWarnSuggestion -->
    <UseVirtualList v-show="!showErrorSuggestion && !showWarnSuggestion && !showSearchIcon && !showHistory"
        ref="virtualListRef" :list="props.wordOptions" :options="{ itemHeight: 30, overscan: 20 }" height="calc(100%)"
        class="list-container">
        <template #default="{ data, index }">
            <div class="item-content clickable-row" :class="{ 'is-selected': selectedWord === data }"
                :style="{ height: '30px' }" @click="handleWordClick(data)">
                <!-- Added a wrapping class 'truncated-text' to ensure uniform string styling -->
                <el-text class="truncated-text" :title="data">
                    {{ data }}
                    <!-- row {{ index }} - {{ data }} -->
                </el-text>
            </div>
        </template>
    </UseVirtualList>
    <UseVirtualList v-show="showHistory" ref="historyVirtualListRef" :list="props.searchHistory.map(item => item.word)"
        :options="{ itemHeight: 30, overscan: 20 }" height="calc(100%)" class="list-container">
        <template #default="{ data, index }">
            <div class="item-content clickable-row" :class="{ 'is-selected': selectedWord === data }"
                :style="{ height: '30px' }" @click="handleWordClick(data)">
                <!-- Added a wrapping class 'truncated-text' to ensure uniform string styling -->
                <el-text class="truncated-text" :title="data">
                    {{ data }}
                    <!-- row {{ index }} - {{ data }} -->
                </el-text>
            </div>
        </template>
    </UseVirtualList>
</template>

<script lang="ts" setup>
import { ref, watch, computed } from 'vue'
import type { PropType } from 'vue'
import { UseVirtualList } from '@vueuse/components'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import { getDictSettingsForLookup } from '@/common/utility'
import type { WordInfoWithLastSearch } from '@/common/type-interface'
import ThreeDotsLoader from '@/components/Svgs/ThreeDotsLoader.vue'


const props = defineProps({
    webSocket: {
        type: [SessionWebSocketService, null],
        required: true
    },
    sessionConfig: {
        type: Object as () => SessionConfig,
        required: true,
        default: () => ({})
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

const virtualListRef = ref<InstanceType<typeof UseVirtualList> | null>(null)
const historyVirtualListRef = ref<InstanceType<typeof UseVirtualList> | null>(null)
const selectedWord = ref<string | null>(null)
const showHistory = computed(() => {
    return !props.keyword.trim()
})

const showErrorSuggestion = computed(() => {
    return (props.wordOptions.length === 1 && props.wordOptions[0].startsWith('FSTD_ERROR'))
})

const showWarnSuggestion = computed(() => {
    return (props.wordOptions.length === 1 && props.wordOptions[0].startsWith('FSTD_WARN'))
})

const showSearchIcon = computed(() => {
    return (props.wordOptions.length === 1 && props.wordOptions[0].startsWith('FSTD_SEARCHING'))
})


watch(() => props.keyword, (newVal) => {
    selectedWord.value = newVal
})

watch(
    () => props.wordOptions,
    () => {
        if (virtualListRef.value?.$el) {
            virtualListRef.value.$el.scrollTop = 0
        }
    },
    { deep: true }
)

const handleWordClick = (word: string) => {
    selectedWord.value = word
    props.webSocket?.sendLookupKeyword(word, props.sessionConfig.default_folder.id, getDictSettingsForLookup(props.sessionConfig.dict_setting_option_name), true)
}
</script>

<style scoped>
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
    /* Crucial: Prevents children from expanding past flex bounds */
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

/* New CSS Rule to force text onto a single line and show "..." when cramped */
.truncated-text {
    display: block !important;
    /* Overrides el-text's default inline layout */
    width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
</style>
