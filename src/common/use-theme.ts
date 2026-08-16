import { ref, watch } from 'vue';
import { useSystemConfigStore } from '@/stores/stores';
import { invoke } from '@tauri-apps/api/core';
import { useRoute } from 'vue-router'


export const useTheme = () => {

    const route = useRoute()
    const isTauriEnv = (): boolean => {
        const envFromRoute = route.query.env as string || ''
        return envFromRoute === '' || envFromRoute === "helper_main_tauri" || envFromRoute === "selection_float_search";
    }

    const systemConfigStore = useSystemConfigStore()
    watch(() => systemConfigStore.systemConfig?.appearance.theme, () => {
        updateTheme();
    })

    const getOperationSystemTheme = (): string => {
        return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }

    const operationSystemTheme = ref(getOperationSystemTheme());

    const updateTauriTheme = (theme: string) => {
        try {
            if (isTauriEnv()) {
                if (theme === "dark") {
                    invoke("set_theme", { theme: "Dark" })
                } else if (theme === "light") {
                    invoke("set_theme", { theme: "Light" })
                } else {
                    invoke("set_theme", { theme: "Auto" })
                }
            }
        }
        catch (error) {
            console.error("Failed to update tauri app theme :", error);
        }
    }

    const updateTheme = () => {
        const theme = systemConfigStore.systemConfig?.appearance.theme;
        if (theme) {
            if (theme === 'auto') {
                const isDark = operationSystemTheme.value === 'dark';
                updateTauriTheme(theme)
                document.documentElement.classList.toggle('dark', isDark);
                systemConfigStore.setIsDark(isDark);
            } else {
                const isDark = theme === 'dark';
                updateTauriTheme(theme)
                document.documentElement.classList.toggle('dark', theme === 'dark');
                systemConfigStore.setIsDark(isDark);
            }
        }
    }

    // 初始化主题
    const initTheme = () => {
        updateTheme();
    };

    // 监听系统主题变化
    const watchSystemTheme = () => {
        window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', e => {
            operationSystemTheme.value = e.matches ? 'dark' : 'light';
            const isDark = operationSystemTheme.value === 'dark';
            const theme = systemConfigStore.systemConfig?.appearance.theme;
            systemConfigStore.setIsDark(isDark);
            if (theme === 'auto') {
                updateTauriTheme(theme)
                document.documentElement.classList.toggle('dark', isDark);
            }
        });
    };

    return {
        initTheme,
        watchSystemTheme,
    };
};

