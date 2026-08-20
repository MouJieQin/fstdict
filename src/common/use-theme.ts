import { ref, watch } from 'vue'
import { useSystemConfigStore } from '@/stores/systemConfig'
import { invoke } from '@tauri-apps/api/core'
import { useRoute } from 'vue-router'

const TAURI_ENV_VALUES = new Set(['', 'helper_main_tauri', 'selection_float_search'])

function isTauriEnvironment(env: string): boolean {
    return TAURI_ENV_VALUES.has(env)
}

function getSystemTheme(): 'dark' | 'light' {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function applyDocumentTheme(isDark: boolean): void {
    document.documentElement.classList.toggle('dark', isDark)
}

export function useTheme() {
    const systemConfigStore = useSystemConfigStore()
    const systemTheme = ref<'dark' | 'light'>(getSystemTheme())
    const route = useRoute()

    const updateTauriTheme = async (theme: string): Promise<void> => {
        const envFromRoute = route.query.env as string || ''
        const isTauri = isTauriEnvironment(envFromRoute)
        if (!isTauri) return
        try {
            const mapped = theme === 'dark' ? 'Dark' : theme === 'light' ? 'Light' : 'Auto'
            await invoke('set_theme', { theme: mapped })
        } catch (error) {
            console.error('Failed to update Tauri theme:', error)
        }
    }

    const applyTheme = (): void => {
        const theme = systemConfigStore.systemConfig?.appearance?.theme

        if (!theme) return

        let isDark: boolean

        if (theme === 'auto') {
            isDark = systemTheme.value === 'dark'
        } else {
            isDark = theme === 'dark'
        }

        applyDocumentTheme(isDark)
        systemConfigStore.setIsDark(isDark)
        updateTauriTheme(theme)
    }

    const initTheme = (): void => {
        applyTheme()
    }

    const watchSystemTheme = (): void => {
        const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

        const handler = (e: MediaQueryListEvent) => {
            systemTheme.value = e.matches ? 'dark' : 'light'

            const theme = systemConfigStore.systemConfig?.appearance?.theme
            if (theme === 'auto') {
                const isDark = systemTheme.value === 'dark'
                applyDocumentTheme(isDark)
                systemConfigStore.setIsDark(isDark)
                updateTauriTheme(theme)
            }
        }

        mediaQuery.addEventListener('change', handler)
    }

    // React to config changes
    watch(
        () => systemConfigStore.systemConfig?.appearance?.theme,
        () => applyTheme()
    )

    return {
        initTheme,
        watchSystemTheme,
    }
}