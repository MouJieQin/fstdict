import { defineStore } from 'pinia'
import type { FolderConfig } from '@/common/type-interface'


export const useFolderConfigStore = defineStore('folderConfig', {
    state: () => ({
        folderConfig: null as FolderConfig | null,
    }),
    actions: {
        setFolderConfig(folderConfig: FolderConfig) {
            this.folderConfig = folderConfig
        },
    }
})

export const useDictConfigStore = defineStore('dictConfig', {
    state: () => ({
        dictConfig: null as any | null,
    }),
    actions: {
        setDictConfig(dictConfig: any) {
            this.dictConfig = dictConfig
        }
    }
})

export const useSystemConfigStore = defineStore('systemConfig', {
    state: () => ({
        systemConfig: null as any | null,
        isDark: false as boolean,
    }),
    actions: {
        setSystemConfig(systemConfig: any) {
            this.systemConfig = systemConfig
        },
        setIsDark(isDark: boolean) {
            this.isDark = isDark
        }
    }
})