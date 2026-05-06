<template>
    <div class="favorite-words-table-container">
        <div class="favorite-folder-name-title">{{ folderName }}</div>
        Total: {{ localFavoriteWords.length }} words
        <el-table v-if="localFavoriteWords" class="favorite-words-table" :data="localFavoriteWords" stripe style="font-size: 1rem;">
            <!-- <el-table-column fixed prop="created_at" label="Favorite At" width="130" show-overflow-tooltip /> -->
            <el-table-column fixed="left" label="Operations" width="130">
                <template #default="scope">
                    <el-button-group>
                        <el-button :icon="BsHeartbreak" size="small"
                            @click="handleUnFavorite(scope.$index, scope.row)" />
                        <el-button :icon="BsSearch" size="small" @click="handleSearch(scope.$index, scope.row)" />
                    </el-button-group>
                </template>
            </el-table-column>
            <el-table-column fixed prop="word" label="Word" show-overflow-tooltip sortable/>
            <el-table-column prop="query_count" label="Query Count" sortable/>
            <el-table-column prop="favorited_at" label="Favorite At" show-overflow-tooltip sortable />
        </el-table>
        <!-- make it show better -->
    </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import type { PropType } from 'vue'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import type { WordInfoWithFavoriteAt } from '@/common/type-interface'
import { BsHeartbreak, BsSearch } from 'vue-icons-plus/bs'

const props = defineProps({
    favoriteWordsDialogVisible: {
        type: Boolean,
        required: true
    },
    webSocket: {
        type: [SessionWebSocketService, null],
        required: true
    },
    folderName: {
        type: String,
        required: true
    },
    favoriteWords: {
        type: Array as PropType<WordInfoWithFavoriteAt[]>,
        required: true
    },
    folderId: {
        type: Number,
        required: true,
        default: 0
    },
})

const emits = defineEmits<{
    (e: 'update-visible', visible: boolean): void
}>()

const localFavoriteWords = ref<WordInfoWithFavoriteAt[]>([...props.favoriteWords])
watch(() => props.favoriteWords, (newVal) => {
    localFavoriteWords.value = [...newVal]
})

const handleUnFavorite = (_: number, row: WordInfoWithFavoriteAt) => {
    props.webSocket?.sendToggleFavor(row.word, props.folderId)
}

const handleSearch = (_: number, row: WordInfoWithFavoriteAt) => {
    props.webSocket?.sendLookupKeywordRequest(row.word)
    emits('update-visible', false)
}


</script>