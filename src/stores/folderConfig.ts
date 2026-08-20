import { defineStore } from 'pinia'
import type { FolderConfig } from '@/common/type-interface'

export const useFolderConfigStore = defineStore('folderConfig', {
    state: (): { folderConfig: FolderConfig | null } => ({
        folderConfig: null,
    }),
    actions: {
        setFolderConfig(folderConfig: FolderConfig) {
            this.folderConfig = folderConfig
        },
    },
})