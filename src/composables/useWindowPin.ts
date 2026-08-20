import { watch, ref, onMounted } from 'vue'
import type { Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { SessionWebSocketService } from '@/common/session-websocket-client'
import type { SessionConfig } from '@/common/type-interface'
import { safeDeepClone } from '@/common/utility'
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
        return e === '' || e === 'selection_float_search' || e === 'helper_main_tauri' || e === 'iwin'
    }

    const applyPinState = async (pinned: boolean): Promise<void> => {
        const e = env()

        if (e === 'selection_float_search') {
            await invoke('set_selection_window_pinned', { pinned })
        } else if (e === 'helper_main_tauri') {
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
        if (e === 'iwin') {
            webSocket.value?.sendFloatingWindowPinClick(sessionId(), newPinned)
        }
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