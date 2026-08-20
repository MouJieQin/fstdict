<template>
    <div class="favorite-words-table-container">
        <div class="favorite-folder-name-title">{{ folderName }}</div>
        <div class="word-count">{{ $t('favoriteWords.totalWords', { count: words.length }) }}</div>

        <el-table v-if="words.length > 0" class="favorite-words-table" :data="words" stripe style="font-size: 1rem">
            <el-table-column fixed="left" :label="$t('favoriteWords.actions')" width="130">
                <template #default="{ row }">
                    <el-button-group>
                        <el-button :icon="BsHeartbreak" size="small" @click="removeFavorite(row)"
                            :aria-label="$t('favoriteWords.removeFromFavorites')" />
                        <el-button :icon="BsSearch" size="small" @click="lookupWord(row)"
                            :aria-label="$t('favoriteWords.lookupWord')" />
                    </el-button-group>
                </template>
            </el-table-column>

            <el-table-column fixed prop="word" :label="$t('favoriteWords.word')" show-overflow-tooltip sortable />
            <el-table-column prop="query_count" :label="$t('favoriteWords.queryCount')" sortable />
            <el-table-column prop="favorited_at" :label="$t('favoriteWords.favoritedAt')" show-overflow-tooltip
                sortable />
        </el-table>

        <p v-else class="empty-state">{{ $t('favoriteWords.empty') }}</p>
    </div>
</template>

<script setup lang="ts">
import type { PropType } from 'vue'
import { computed } from 'vue'
import { BsHeartbreak, BsSearch } from 'vue-icons-plus/bs'

import type { SessionWebSocketService } from '@/common/session-websocket-client'
import type { WordInfoWithFavoriteAt } from '@/common/type-interface'

const props = defineProps({
    favoriteWordsDialogVisible: {
        type: Boolean,
        required: true,
    },
    webSocket: {
        type: [Object, null] as PropType<SessionWebSocketService | null>,
        required: true,
    },
    folderName: {
        type: String,
        required: true,
    },
    favoriteWords: {
        type: Array as PropType<WordInfoWithFavoriteAt[]>,
        required: true,
    },
    folderId: {
        type: Number,
        required: true,
        default: 0,
    },
})

const emit = defineEmits<{
    (e: 'update-visible', visible: boolean): void
}>()

const words = computed(() => props.favoriteWords)

function removeFavorite(row: WordInfoWithFavoriteAt): void {
    props.webSocket?.sendToggleFavor(row.word, props.folderId)
}

function lookupWord(row: WordInfoWithFavoriteAt): void {
    props.webSocket?.sendLookupKeywordRequest(row.word)
    emit('update-visible', false)
}
</script>

<style scoped>
.favorite-words-table-container {
    margin: 20px auto 0;
    max-width: 960px;
}

.favorite-folder-name-title {
    font-size: 1.3rem;
    font-weight: bold;
    margin-bottom: 10px;
    text-align: center;
}

.word-count {
    text-align: center;
    color: var(--el-text-color-secondary);
    margin-bottom: 16px;
}

.favorite-words-table {
    max-height: 80%;
}

.empty-state {
    text-align: center;
    color: var(--el-text-color-secondary);
    padding: 40px 0;
}
</style>