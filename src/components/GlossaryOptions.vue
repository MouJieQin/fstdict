<template>
    <div class="glossary-panel">
        <div class="glossary-panel-header">
            <div class="gloassary-title">
                <el-text truncated class="title-text">
                    {{ glossaryName }}
                </el-text>
                <el-text class="title-text">
                    {{ `(${glossarySize})` }}
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
                        <ArrowDown />
                    </el-icon>
                    <template #dropdown>
                        <el-dropdown-menu class="vibrant-dropdown">
                            <el-dropdown-item v-for="method in sortMethods" :key="method"
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

                <!-- Search: click the icon to reveal an input; the input
                     filters the list with the selected search method. -->
                <el-icon class="clickable-icon" @click="searchVisible = !searchVisible">
                    <Search v-if="!searchVisible" />
                    <Close v-else />
                </el-icon>
            </div>
        </div>

        <div class="glossary-search-bar" v-if="searchVisible">
            <el-input v-model="search.query" size="small" clearable autocomplete="off" autocorrect="off"
                autocapitalize="off" spellcheck="false" :placeholder="t('glossaryOptions.searchPlaceholder')"
                class="glossary-search-input">
                <template #prefix>
                    <el-icon>
                        <Search />
                    </el-icon>
                </template>
            </el-input>
            <el-dropdown trigger="click" placement="bottom-end" @command="handleSearchMethodCommand"
                popper-class="vibrant-dropdown">
                <!-- <el-button size="small" text class="glossary-search-method-btn"> -->
                <!-- <span>{{ searchMethodLabel }}</span> -->
                <el-icon class="clickable-icon">
                    <ArrowDown />
                </el-icon>
                <!-- </el-button> -->
                <template #dropdown>
                    <el-dropdown-menu class="vibrant-dropdown">
                        <el-dropdown-item v-for="m in searchMethodOptions" :key="m.value"
                            :class="{ 'is-active': m.value === search.method }" :command="{ method: m.value }">
                            <el-icon v-if="m.value === search.method" style="color: var(--el-color-primary)">
                                <Check />
                            </el-icon>
                            <el-icon v-else style="visibility: hidden;">
                                <Check />
                            </el-icon>
                            <span>{{ m.label }}</span>
                        </el-dropdown-item>
                    </el-dropdown-menu>
                </template>
            </el-dropdown>
        </div>

        <div class="glossary-options">
            <UseVirtualList :list="search.filteredWords" :options="{ itemHeight: ITEM_HEIGHT, overscan: 20 }"
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
import { computed, nextTick, reactive, ref, watch } from 'vue'
import type { PropType } from 'vue'
import { UseVirtualList } from '@vueuse/components'

import { SessionWebSocketService } from '@/common/session-websocket-client'
import type { SessionConfig, WordInfo, WordInfoWithNoteUpdateAt, WordInfoWithFavoriteAt } from '@/common/type-interface'
import { getDictSettingsForLookup } from '@/common/utility'
import {
    GLOSSARY_SEARCH_METHOD,
    useGlossarySearch,
    type GlossarySearchMethod,
} from '@/common/useGlossarySearch'

import { MAIN_MENU_INDEX } from '@/common/constants'

import { BsSortDown, BsSortUpAlt } from 'vue-icons-plus/bs'
import { Check, Search, Close, ArrowDown } from '@element-plus/icons-vue'

import { useI18n } from 'vue-i18n'
const { t } = useI18n()


const WORD_SORT_METHOD = {
    LAST_SEARCH: t('glossaryOptions.lastSearch'),
    SEARCH_COUNT: t('glossaryOptions.searchCount'),
    ALPHABET: t('glossaryOptions.alphabetical'),
} as const

const FAVORITE_SORT_METHOD = {
    ...WORD_SORT_METHOD,
    ADD_TIME: t('glossaryOptions.addTime'),
} as const

const NOTE_SORT_METHOD = {
    ...WORD_SORT_METHOD,
    UPDATE_TIME: t('glossaryOptions.updateTime'),
} as const

const sortDescending = ref<boolean>(true)
const sortMethods = ref<string[]>([])
const activeSortMethod = ref<string>(WORD_SORT_METHOD.LAST_SEARCH)

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
    searchHistory: {
        type: Array as PropType<WordInfo[]>,
        default: () => [],
    },
    noteWords: {
        type: Array as PropType<WordInfoWithNoteUpdateAt[]>,
        default: () => [],
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
})

const emit = defineEmits<{
    (e: 'select', word: string): void
}>()

// --- Refs ---
const listRef = ref<InstanceType<typeof UseVirtualList> | null>(null)
const selectedWord = ref<string | null>(null)
const glossaryName = ref<string>('')
const glossarySize = ref<number>(0)

const displayList = computed(() => {
    if (props.activeMenuIndex === MAIN_MENU_INDEX.HISTORY) {
        glossaryName.value = t('glossaryOptions.history')
        glossarySize.value = historyWords.value.length
        sortMethods.value = Object.values(WORD_SORT_METHOD)
        return historyWords.value.map((item) => item.word)
    } else if (props.activeMenuIndex === MAIN_MENU_INDEX.NOTES) {
        glossaryName.value = t('glossaryOptions.notes')
        glossarySize.value = props.noteWords.length
        sortMethods.value = Object.values(NOTE_SORT_METHOD)
        return noteWords.value.map((item) => item.word)
    }
    glossaryName.value = props.folderName
    glossarySize.value = props.favoriteWords.length
    sortMethods.value = Object.values(FAVORITE_SORT_METHOD)
    return favoriteWords.value.map((item) => item.word)
})

const historyWords = computed(() => {
    if (activeSortMethod.value === WORD_SORT_METHOD.SEARCH_COUNT) {
        return sortDescending.value ? props.searchHistory.sort((a, b) => b.query_count - a.query_count) : props.searchHistory.sort((a, b) => a.query_count - b.query_count)
    } else if (activeSortMethod.value === WORD_SORT_METHOD.ALPHABET) {
        return sortDescending.value ? props.searchHistory.sort((a, b) => b.word.localeCompare(a.word)) : props.searchHistory.sort((a, b) => a.word.localeCompare(b.word))
    }
    else {
        return sortDescending.value ? props.searchHistory.sort((a, b) => Date.parse(b.last_searched ?? '') - Date.parse(a.last_searched ?? '')) : props.searchHistory.sort((a, b) => Date.parse(a.last_searched ?? '') - Date.parse(b.last_searched ?? ''))
    }
})

const noteWords = computed(() => {
    if (activeSortMethod.value === NOTE_SORT_METHOD.UPDATE_TIME) {
        return sortDescending.value ? props.noteWords.sort((a, b) => Date.parse(b.updated_at ?? '') - Date.parse(a.updated_at ?? '')) : props.noteWords.sort((a, b) => Date.parse(a.updated_at ?? '') - Date.parse(b.updated_at ?? ''))
    } else if (activeSortMethod.value === FAVORITE_SORT_METHOD.SEARCH_COUNT) {
        return sortDescending.value ? props.noteWords.sort((a, b) => b.query_count - a.query_count) : props.noteWords.sort((a, b) => a.query_count - b.query_count)
    } else if (activeSortMethod.value === FAVORITE_SORT_METHOD.ALPHABET) {
        return sortDescending.value ? props.noteWords.sort((a, b) => b.word.localeCompare(a.word)) : props.noteWords.sort((a, b) => a.word.localeCompare(b.word))
    }
    else {
        return sortDescending.value ? props.noteWords.sort((a, b) => Date.parse(b.last_searched ?? '') - Date.parse(a.last_searched ?? '')) : props.noteWords.sort((a, b) => Date.parse(a.last_searched ?? '') - Date.parse(b.last_searched ?? ''))
    }
})

const favoriteWords = computed(() => {
    if (activeSortMethod.value === FAVORITE_SORT_METHOD.ADD_TIME) {
        if (sortDescending.value) {
            return props.favoriteWords.sort((a, b) => Date.parse(b.favorited_at ?? '') - Date.parse(a.favorited_at ?? ''))
        }
        return props.favoriteWords.sort((a, b) => Date.parse(a.favorited_at ?? '') - Date.parse(b.favorited_at ?? ''))
    } else if (activeSortMethod.value === FAVORITE_SORT_METHOD.SEARCH_COUNT) {
        return sortDescending.value ? props.favoriteWords.sort((a, b) => b.query_count - a.query_count) : props.favoriteWords.sort((a, b) => a.query_count - b.query_count)
    } else if (activeSortMethod.value === FAVORITE_SORT_METHOD.ALPHABET) {
        return sortDescending.value ? props.favoriteWords.sort((a, b) => b.word.localeCompare(a.word)) : props.favoriteWords.sort((a, b) => a.word.localeCompare(b.word))
    } else {
        return sortDescending.value ? props.favoriteWords.sort((a, b) => Date.parse(b.last_searched ?? '') - Date.parse(a.last_searched ?? '')) : props.favoriteWords.sort((a, b) => Date.parse(a.last_searched ?? '') - Date.parse(b.last_searched ?? ''))
    }
})

// --- Search (Fuzzy / Contains / Regex) ---
// Bound to the sorted displayList; fuzzy mode re-ranks by relevance while
// contains/regex keep the source sort order.
const search = reactive(useGlossarySearch(displayList))

const searchVisible = ref(false)

const searchMethodOptions = computed(() => [
    { value: GLOSSARY_SEARCH_METHOD.FUZZY, label: t('glossaryOptions.fuzzy') },
    { value: GLOSSARY_SEARCH_METHOD.CONTAINS, label: t('glossaryOptions.contains') },
    { value: GLOSSARY_SEARCH_METHOD.REGEX, label: t('glossaryOptions.regex') },
])

const searchMethodLabel = computed(
    () => searchMethodOptions.value.find((m) => m.value === search.method)?.label ?? ''
)

const handleSearchMethodCommand = (command: { method: GlossarySearchMethod }): void => {
    search.method = command.method
}

// --- Virtual list height follows the container (el-splitter is resizable) ---
// `height: 100%` / `calc(100%)` cannot work here: the panel's computed
// height comes from flex-basis, not an explicit `height`, so percentage
// resolution against it is undefined and the list collapses. Instead we
// measure the wrapper with a ResizeObserver and feed real px to
// UseVirtualList, which also keeps the list live while the splitter drags.

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
