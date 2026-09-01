import { watch, ref, onMounted, computed } from 'vue'
import type { Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { SessionWebSocketService } from '@/common/session-websocket-client'
import { safeDeepClone } from '@/common/utility'
import { ENV, TAURI_CMD } from '@/common/constants'
import { useRoute } from 'vue-router'


interface UseWindowPinOptions {
    systemConfig: Ref<any>
    webSocket: Ref<SessionWebSocketService | null>
}

export function useWindowPin(options: UseWindowPinOptions) {
    const { webSocket, systemConfig } = options

    const tauriWindow = ref<ReturnType<typeof getCurrentWindow> | null>(null)
    const route = useRoute()

    const env = (): string => {
        return route.query.env as string || ''
    }

    const showPinButton = (): boolean => {
        const e = env()
        return e === ENV.MAIN || e === ENV.SELECTION || e === ENV.HELPER
    }

    const applyPinState = async (pinned: boolean): Promise<void> => {
        const e = env()

        if (e === ENV.SELECTION) {
            await invoke(TAURI_CMD.SET_SELECTION_WINDOW_PINNED, { pinned })
        } else if (e === ENV.HELPER) {
            await invoke(TAURI_CMD.SET_MAIN_WINDOW_PINNED, { pinned })
        } else if (e === ENV.MAIN) {
            await getCurrentWindow().setAlwaysOnTop(pinned)
        }
    }

    const togglePin = async () => {
        const newPinned = !isPinned.value
        const e = env()
        const config = safeDeepClone(systemConfig.value)
        if (e === ENV.SELECTION) {
            config.app.windows.helper_selection.pinned = newPinned
        } else if (e === ENV.HELPER) {
            config.app.windows.helper_main.pinned = newPinned
        } else if (e === ENV.MAIN) {
            config.app.windows.main.pinned = newPinned
        }
        webSocket.value?.sendUpdateSystemConfig(config)
    }

    const isPinned = computed(() => {
        const e = env()
        const config = safeDeepClone(systemConfig.value)
        try {
            if (e === ENV.SELECTION) {
                return config.app.windows.helper_selection.pinned
            } else if (e === ENV.HELPER) {
                return config.app.windows.helper_main.pinned
            } else if (e === ENV.MAIN) {
                return config.app.windows.main.pinned
            }
        } catch (e) {
            return false
        }
        return false
    })

    watch(isPinned, async (val) => {
        await applyPinState(val)
    })

    onMounted(async () => {
        if (env() === ENV.MAIN) {
            tauriWindow.value = getCurrentWindow()
            await applyPinState(isPinned.value)
        }
    })

    return {
        showPinButton,
        isPinned,
        togglePin
    }
}