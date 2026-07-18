import { ref, watch } from 'vue';
import { useSystemConfigStore } from '@/stores/stores';

export const useTheme = () => {

    const systemConfigStore = useSystemConfigStore();

    watch(() => systemConfigStore.systemConfig?.appearance.theme, () => {
        updateTheme();
    })

    const getOperationSystemTheme = (): string => {
        return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }

    const operationSystemTheme = ref(getOperationSystemTheme());

    const updateTheme = () => {
        const theme = systemConfigStore.systemConfig?.appearance.theme;
        if (theme) {
            if (theme === 'auto') {
                document.documentElement.classList.toggle('dark', operationSystemTheme.value === 'dark');
            } else {
                document.documentElement.classList.toggle('dark', theme === 'dark');
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
            const theme = systemConfigStore.systemConfig?.appearance.theme;
            if (theme === 'auto') {
                document.documentElement.classList.toggle('dark', operationSystemTheme.value === 'dark');
            }
        });
    };

    return {
        initTheme,
        watchSystemTheme,
    };
};

