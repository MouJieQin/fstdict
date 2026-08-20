import { ref, computed } from 'vue'
import { ElMessageBox } from 'element-plus'
import type { SessionWebSocketService } from '@/common/session-websocket-client'
import type { SessionConfig, SessionNameId } from '@/common/type-interface'
import { getDefaultSessionConfig } from '@/common/utility'
import { useSystemConfigStore } from '@/stores/systemConfig'
import { useRouter } from 'vue-router'
import { safeDeepClone } from '@/common/utility'


const MAX_SESSION_NAME_LENGTH = 30

export function useSessionManagement(
    webSocket: () => SessionWebSocketService | null,
    currentSessionId: () => number,
    env: () => string
) {
    const router = useRouter()
    const systemConfigStore = useSystemConfigStore()

    const sessionsNameId = ref<SessionNameId[]>([])

    const redirectSession = (sessionId: number): void => {
        const systemConfig = safeDeepClone(systemConfigStore.systemConfig)
        const envValue = env()

        if (envValue === '') {
            systemConfig.app.session.id = sessionId
            webSocket()?.sendUpdateSystemConfig(systemConfig)
        } else if (envValue === 'helper_main_tauri') {
            systemConfig.ocr.session.id = sessionId
            webSocket()?.sendUpdateSystemConfig(systemConfig)
        } else if (envValue === 'selection_float_search') {
            systemConfig.app.helper_selection.session.id = sessionId
            webSocket()?.sendUpdateSystemConfig(systemConfig)
        }

        router.push({
            path: `/dict/${sessionId}`,
            query: { env: envValue },
        })
    }

    const createSession = async (): Promise<void> => {
        try {
            const { value } = await ElMessageBox.prompt(
                'Enter a name for the new session',
                'Create Session',
                {
                    confirmButtonText: 'Create',
                    cancelButtonText: 'Cancel',
                    inputValidator: (value: string) => {
                        if (!value?.trim()) return 'Session name cannot be empty'
                        if (value.length > MAX_SESSION_NAME_LENGTH) {
                            return `Name must be under ${MAX_SESSION_NAME_LENGTH} characters`
                        }
                        return true
                    },
                }
            )
            webSocket()?.sendCreateSession(getDefaultSessionConfig(value.trim()))
        } catch {
            // User cancelled
        }
    }

    const renameSession = async (): Promise<void> => {
        try {
            const { value } = await ElMessageBox.prompt(
                'Rename the current session',
                'Rename Session',
                {
                    confirmButtonText: 'Rename',
                    cancelButtonText: 'Cancel',
                    inputValidator: (value: string) => {
                        if (value.length > MAX_SESSION_NAME_LENGTH) {
                            return `Name must be under ${MAX_SESSION_NAME_LENGTH} characters`
                        }
                        return true
                    },
                }
            )
            webSocket()?.sendRenameSession(value.trim())
        } catch {
            // User cancelled
        }
    }

    const removeSession = async (): Promise<void> => {
        try {
            await ElMessageBox.confirm(
                'Are you sure you want to remove the current session?',
                'Remove Session',
                {
                    confirmButtonText: 'Remove',
                    cancelButtonText: 'Cancel',
                    type: 'warning',
                    center: true,
                }
            )
            webSocket()?.sendRemoveSession()
            redirectSession(1)
        } catch {
            // User cancelled
        }
    }

    const handleSessionCommand = (command: { cmd: string; id: number }): void => {
        switch (command.cmd) {
            case 'switch':
                redirectSession(command.id)
                break
            case 'create':
                createSession()
                break
            case 'rename':
                renameSession()
                break
            case 'remove':
                removeSession()
                break
        }
    }

    const currentSessionName = computed(() => {
        const id = currentSessionId()
        return sessionsNameId.value.find(s => s.id === id)?.name || ''
    })

    return {
        sessionsNameId,
        redirectSession,
        createSession,
        renameSession,
        removeSession,
        handleSessionCommand,
        currentSessionName,
    }
}