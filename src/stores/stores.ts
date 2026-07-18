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

// export const useSystemConfigStore = defineStore('systemConfig', {
//     state: () => ({
//         systemConfig: null as any | null,
//         updateAppearanceTheme: null as unknown as (theme: string) => void | null,
//         updateSystemConfig: null as unknown as (systemConfig: any) => void | null
//     }),
//     actions: {
//         setAppearanceTheme(theme: string) {
//             if (this.systemConfig && this.systemConfig.appearance.theme !== theme) {
//                 console.log('this.systemConfig.appearance.theme:', this.systemConfig.appearance.theme)
//                 console.log('theme:', theme)
//                 this.systemConfig.appearance.theme = theme
//                 this.updateSystemConfig(this.systemConfig)
//             }
//         },
//         setSystemConfig(systemConfig: any) {
//             if (!this.systemConfig) {
//                 this.systemConfig = systemConfig
//             }
//         },
//         setUpdateAppearanceTheme(updateAppearanceTheme: (theme: string) => void) {
//             this.updateAppearanceTheme = updateAppearanceTheme
//         },
//         setUpdateSystemConfig(updateSystemConfig: (systemConfig: any) => void) {
//             this.updateSystemConfig = updateSystemConfig
//         }
//     }
// })

export const useSystemConfigStore = defineStore('systemConfig', {
    state: () => ({
        systemConfig: null as any | null,
    }),
    actions: {
        setSystemConfig(systemConfig: any) {
            this.systemConfig = systemConfig
        },
    }
})