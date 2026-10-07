<template>
    <div class="glossary-panel">
        <div class="glossary-panel-header">
            <div class="gloassary-title">
                <el-text truncated class="title-text">
                    {{ folderName }}
                </el-text>
                <el-text class="title-text">
                    {{ `(${favoriteWords.length})` }}
                </el-text>
            </div>

            <div class="sort-icon">
                <el-icon class="clickable-icon" @click="sortDescending = !sortDescending">
                    <BsSortDown v-show="sortDescending" />
                    <BsSortUpAlt v-show="!sortDescending" />
                </el-icon>

                <el-dropdown trigger="click" placement="bottom-end" @command="handleSortCommand"
                    popper-class="vibrant-dropdown">
                    <el-icon class="clickable-icon">
                        <Sort />
                    </el-icon>
                    <template #dropdown>
                        <el-dropdown-menu class="vibrant-dropdown">
                            <el-dropdown-item v-for="method in favoriteSortMethods" :key="method"
                                :class="{ 'is-active': method === activeSortMethod }" :command="{ method: method }">
                                <el-icon v-if="method == activeSortMethod" style="color: var(--el-color-primary)">
                                    <Check />
                                </el-icon>
                                <el-icon v-else style="visibility: hidden;">
                                    <Check />
                                </el-icon>
                                <span>{{ method }}</span>
                            </el-dropdown-item>
                        </el-dropdown-menu>
                    </template>
                </el-dropdown>
            </div>
        </div>
        <div class="glossary-options">
            <UseVirtualList ref="listRef" :list="displayList" :options="{ itemHeight: ITEM_HEIGHT, overscan: 20 }"
                height="calc(100%)" class="list-container">
                <template #default="{ data, index }">
                    <div class="clickable-row" :class="{ 'is-selected': selectedWord === data }"
                        :style="{ height: `${ITEM_HEIGHT}px` }" @click="handleWordClick(data)">
                        <el-text truncated class="word-text">
                            {{ data }}
                        </el-text>
                    </div>
                </template>
            </UseVirtualList>
        </div>

    </div>
</template>

<script lang="ts" setup>
import { ref, computed, watch, nextTick } from 'vue'
import type { PropType } from 'vue'
import { UseVirtualList } from '@vueuse/components'

import { SessionWebSocketService } from '@/common/session-websocket-client'
import type { SessionConfig, WordInfoWithLastSearch, WordInfoWithFavoriteAt } from '@/common/type-interface'
import { getDictSettingsForLookup } from '@/common/utility'

import { MAIN_MENU_INDEX } from '@/common/constants'

import { BsSortDown, BsSortUpAlt } from 'vue-icons-plus/bs'
import { Sort, Check } from '@element-plus/icons-vue'

const FAVORITE_SORT_METHOD = {
    TIME: 'time',
    WORD: 'word',
    QUERY: 'query',
} as const

const sortDescending = ref<boolean>(true)
const favoriteSortMethods: string[] = [FAVORITE_SORT_METHOD.TIME, FAVORITE_SORT_METHOD.WORD, FAVORITE_SORT_METHOD.QUERY]
const activeSortMethod = ref<string>(FAVORITE_SORT_METHOD.TIME)

const handleSortCommand = (command: { method: string }): void => {
    activeSortMethod.value = command.method
}




const ITEM_HEIGHT = 30

const props = defineProps({
    activeMenuIndex: {
        type: String,
        default: '',
    },
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
    folderName: {
        type: String,
        required: false,
        default: '',
    },
    favoriteWords: {
        type: Array as PropType<WordInfoWithFavoriteAt[]>,
        required: false,
        default: () => [],
    },
    searchHistory: {
        type: Array as PropType<WordInfoWithLastSearch[]>,
        default: () => [],
    },
})

const emit = defineEmits<{
    (e: 'select', word: string): void
}>()

// --- Refs ---
const listRef = ref<InstanceType<typeof UseVirtualList> | null>(null)
const selectedWord = ref<string | null>(null)

const displayList = computed(() => {
    if (props.activeMenuIndex === MAIN_MENU_INDEX.HISTORY) {
        return props.searchHistory.map((item) => item.word)
    }
    // return props.favoriteWords.map((item) => item.word)
    return favoriteWords.value.map((item) => item.word)
})

const favoriteWords = computed(() => {
    if (activeSortMethod.value === FAVORITE_SORT_METHOD.TIME) {
        if (sortDescending.value) {
            return props.favoriteWords.sort((a, b) => Date.parse(b.favorited_at ?? '') - Date.parse(a.favorited_at ?? ''))
        }
        return props.favoriteWords.sort((a, b) => Date.parse(a.favorited_at ?? '') - Date.parse(b.favorited_at ?? ''))
    } else if (activeSortMethod.value === FAVORITE_SORT_METHOD.QUERY) {
        return sortDescending.value ? props.favoriteWords.sort((a, b) => b.query_count - a.query_count) : props.favoriteWords.sort((a, b) => a.query_count - b.query_count)
    } else {
        return sortDescending.value ? props.favoriteWords.sort((a, b) => b.word.localeCompare(a.word)) : props.favoriteWords.sort((a, b) => a.word.localeCompare(b.word))
    }
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
    () => props.favoriteWords,
    () => {
        nextTick(() => {
            const el = listRef.value?.$el as HTMLElement | undefined
            if (el) el.scrollTop = 0
        })
    },
    { deep: true }
)

watch(() => props.webSocket, () => {
    props.webSocket?.sendSearchHistoryRequest()
})
</script>
