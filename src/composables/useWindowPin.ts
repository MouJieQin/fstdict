import { watch, ref, onMounted } from 'vue'
import type { Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { SessionWebSocketService } from '@/common/session-websocket-client'
import type { SessionConfig } from '@/common/type-interface'
import { safeDeepClone } from '@/common/utility'
import { ENV } from '@/common/constants'
import { useRoute } from 'vue-router'


interface UseWindowPinOptions {
    sessionId: () => number
    isPinned: () => boolean
    sessionConfig: Ref<SessionConfig>
    webSocket: Ref<SessionWebSocketService | null>

}

export function useWindowPin(options: UseWindowPinOptions) {
    const { sessionId, isPinned, sessionConfig, webSocket } = options

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
            await invoke('set_selection_window_pinned', { pinned })
        } else if (e === ENV.HELPER) {
            await invoke('set_main_window_pinned', { pinned })
        } else if (tauriWindow.value) {
            await tauriWindow.value.setAlwaysOnTop(pinned)
        }
    }

    const togglePin = (): void => {
        const e = env()
        const newPinned = !isPinned()

        const config = safeDeepClone(sessionConfig.value)
        config.pin = { is_pinned: newPinned }
        webSocket.value?.sendSessionConfig(config)
    }

    watch(isPinned, (val) => {
        applyPinState(val)
    })

    onMounted(async () => {
        if (env() === '') {
            tauriWindow.value = getCurrentWindow()
            await applyPinState(isPinned())
        }
    })

    return {
        showPinButton,
        togglePin,
        applyPinState,
    }
}