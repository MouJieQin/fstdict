import { ref, computed } from 'vue'
import { ElMessageBox } from 'element-plus'
import type { SessionWebSocketService } from '@/common/session-websocket-client'
import type { SessionNameId } from '@/common/type-interface'
import { getDefaultSessionConfig } from '@/common/utility'
import { useSystemConfigStore } from '@/stores/systemConfig'
import { useRouter } from 'vue-router'
import { safeDeepClone } from '@/common/utility'
import { useI18n } from 'vue-i18n'

const MAX_SESSION_NAME_LENGTH = 30

export function useSessionManagement(
    webSocket: () => SessionWebSocketService | null,
    currentSessionId: () => number,
    env: () => string
) {
    const router = useRouter()
    const systemConfigStore = useSystemConfigStore()
    const { t } = useI18n()

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
                t('session.createPrompt'),
                t('session.createTitle'),
                {
                    confirmButtonText: t('session.createButton'),
                    cancelButtonText: t('common.cancel'),
                    inputValidator: (value: string) => {
                        if (!value?.trim()) return t('session.nameEmpty')
                        if (value.length > MAX_SESSION_NAME_LENGTH) {
                            return t('session.nameTooLong', { max: MAX_SESSION_NAME_LENGTH })
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
                t('session.renamePrompt'),
                t('session.renameTitle'),
                {
                    confirmButtonText: t('session.renameButton'),
                    cancelButtonText: t('common.cancel'),
                    inputValidator: (value: string) => {
                        if (value.length > MAX_SESSION_NAME_LENGTH) {
                            return t('session.nameTooLong', { max: MAX_SESSION_NAME_LENGTH })
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
                t('session.removeConfirm'),
                t('session.removeTitle'),
                {
                    confirmButtonText: t('session.removeButton'),
                    cancelButtonText: t('common.cancel'),
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