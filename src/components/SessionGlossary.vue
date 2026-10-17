<template>
    <el-splitter layout="vertical" class="session-glossary">
        <el-splitter-panel>
            <GlossaryOptions :active-menu-index="MAIN_MENU_INDEX.HISTORY" :web-socket="webSocket"
                :session-config="sessionConfig" :keyword="keyword" :folder-name="folderName"
                :favorite-words="favoriteWords" :search-history="searchHistory" />
        </el-splitter-panel>
        <el-splitter-panel :min="50">
            <GlossaryOptions :active-menu-index="MAIN_MENU_INDEX.GLOSSARY_FOLDERS_PREFIX" :web-socket="webSocket"
                :session-config="sessionConfig" :keyword="keyword" :folder-name="folderName"
                :favorite-words="favoriteWords" :search-history="searchHistory" />
        </el-splitter-panel>
    </el-splitter>
</template>

<script lang="ts" setup>
import { ref, computed, watch, nextTick } from 'vue'
import type { PropType } from 'vue'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import type { SessionConfig, WordInfo, WordInfoWithFavoriteAt } from '@/common/type-interface'
import { MAIN_MENU_INDEX } from '@/common/constants'


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
    searchHistory: {
        type: Array as PropType<WordInfo[]>,
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
</script>