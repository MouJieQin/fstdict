import { defineStore } from 'pinia'
import type { FolderWords, WordInfoWithFavoriteAt } from '@/common/type-interface'

export const useFolderWordsStore = defineStore('folderWords', {
    state: (): { folderWords: FolderWords } => ({
        folderWords: [],
    }),
    actions: {
        setFolderWords(folder_id: number, words: WordInfoWithFavoriteAt[]) {
            this.folderWords[folder_id] = words
        },
    },
})