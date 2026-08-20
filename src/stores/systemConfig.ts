import { defineStore } from 'pinia'

interface SystemConfigState {
    systemConfig: any | null
    isDark: boolean
}

export const useSystemConfigStore = defineStore('systemConfig', {
    state: (): SystemConfigState => ({
        systemConfig: null,
        isDark: false,
    }),
    actions: {
        setSystemConfig(systemConfig: any) {
            this.systemConfig = systemConfig
        },
        setIsDark(isDark: boolean) {
            this.isDark = isDark
        },
    },
})