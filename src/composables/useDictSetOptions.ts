import { ref, watch, nextTick, computed } from 'vue'
import { ElMessageBox, ElMessage } from 'element-plus'
import Sortable from 'sortablejs'
import type { Ref } from 'vue'
import type { DictSettingInfo, SessionConfig } from '@/common/type-interface'
import { useDictConfigStore } from '@/stores/dictConfig'
import type { SessionWebSocketService } from '@/common/session-websocket-client'
import { safeDeepClone } from '@/common/utility'
import { useI18n } from 'vue-i18n'

const MAX_OPTION_NAME_LENGTH = 30

interface UseDictSetOptionsParams {
    webSocket: Ref<SessionWebSocketService | null>
    sessionConfig: Ref<SessionConfig>
    listContainerRef: Ref<HTMLElement | null>
}

export function useDictSetOptions(params: UseDictSetOptionsParams) {
    const { webSocket, sessionConfig, listContainerRef } = params
    const { t } = useI18n()

    const dictConfigStore = useDictConfigStore()
    const localDictConfig = ref(safeDeepClone(dictConfigStore.dictConfig))
    const localSessionConfig = ref(safeDeepClone(sessionConfig.value))
    const list = ref<DictSettingInfo[]>([])

    let sortableInstance: Sortable | null = null

    const currentOptionName = computed({
        get: () => localSessionConfig.value.dict_setting_option_name,
        set: (val: string) => {
            localSessionConfig.value.dict_setting_option_name = val
        },
    })

    const isDefaultOption = computed(() => currentOptionName.value === 'default')

    const updateList = (): void => {
        const name = currentOptionName.value
        list.value = localDictConfig.value?.dict_set_options?.[name] || []
    }

    const initSortable = (): void => {
        if (sortableInstance) {
            sortableInstance.destroy()
            sortableInstance = null
        }

        const container = listContainerRef.value
        if (!container) return

        sortableInstance = Sortable.create(container, {
            animation: 300,
            draggable: '.dict-settings-drag-cards',
            ghostClass: 'sortable-ghost',
            forceFallback: true,
            fallbackClass: 'sortable-dragging',
            fallbackOnBody: false,

            onEnd: ({ oldIndex, newIndex }) => {
                if (oldIndex === undefined || newIndex === undefined) return
                if (oldIndex === newIndex) return

                const items = [...list.value]
                const [moved] = items.splice(oldIndex, 1)
                items.splice(newIndex, 0, moved)
                list.value = items

                const name = currentOptionName.value
                if (name && localDictConfig.value?.dict_set_options?.[name]) {
                    localDictConfig.value.dict_set_options[name] = [...items]
                }
            },
        })
    }

    const refresh = async (): Promise<void> => {
        localDictConfig.value = safeDeepClone(dictConfigStore.dictConfig)
        updateList()
        await nextTick()
        initSortable()
    }

    const createOption = async (): Promise<void> => {
        try {
            const { value } = await ElMessageBox.prompt(
                t('dictSettings.createPrompt'),
                t('dictSettings.createTitle'),
                {
                    confirmButtonText: t('dictSettings.createButton'),
                    cancelButtonText: t('common.cancel'),
                    inputValidator: (name: string) => {
                        const options = localDictConfig.value?.dict_set_options || {}
                        if (name in options) return t('dictSettings.nameExists')
                        if (name.length > MAX_OPTION_NAME_LENGTH) {
                            return t('dictSettings.nameTooLong', { max: MAX_OPTION_NAME_LENGTH })
                        }
                        return true
                    },
                }
            )

            syncDictConfigIfChanged()
            webSocket.value?.sendCreateDictSetOption(value.trim())
            currentOptionName.value = value.trim()
            syncSessionConfigIfChanged()
        } catch {
            // User cancelled
        }
    }

    const renameOption = async (): Promise<void> => {
        const oldName = currentOptionName.value

        try {
            const { value } = await ElMessageBox.prompt(
                t('dictSettings.renamePrompt'),
                t('dictSettings.renameTitle'),
                {
                    confirmButtonText: t('dictSettings.renameButton'),
                    cancelButtonText: t('common.cancel'),
                    inputValue: oldName,
                    inputValidator: (name: string) => {
                        const options = localDictConfig.value?.dict_set_options || {}
                        if (name in options && name !== oldName) {
                            return t('dictSettings.nameExists')
                        }
                        if (name.length > MAX_OPTION_NAME_LENGTH) {
                            return t('dictSettings.nameTooLong', { max: MAX_OPTION_NAME_LENGTH })
                        }
                        return true
                    },
                }
            )

            syncDictConfigIfChanged()
            webSocket.value?.sendRenameDictSetOption(oldName, value.trim())
            currentOptionName.value = value.trim()
            syncSessionConfigIfChanged()
        } catch {
            // User cancelled
        }
    }

    const deleteOption = async (): Promise<void> => {
        const name = currentOptionName.value

        try {
            await ElMessageBox.confirm(
                t('dictSettings.deleteConfirm', { name }),
                t('dictSettings.deleteTitle'),
                {
                    confirmButtonText: t('dictSettings.deleteButton'),
                    cancelButtonText: t('common.cancel'),
                    type: 'warning',
                    center: true,
                }
            )

            webSocket.value?.sendRemoveDictSetOption(name)
            currentOptionName.value = 'default'
        } catch {
            // User cancelled
        }
    }

    const deleteDictionary = async (dictName: string): Promise<void> => {
        try {
            await ElMessageBox.confirm(
                t('dictSettings.deleteDictConfirm', { name: dictName }),
                t('dictSettings.deleteDictTitle'),
                {
                    confirmButtonText: t('common.delete'),
                    cancelButtonText: t('common.cancel'),
                    type: 'warning',
                    center: true,
                }
            )
            webSocket.value?.sendDeleteDict(dictName)
        } catch {
            ElMessage({ type: 'info', message: t('titleBar.deleteCancelled') })
        }
    }

    const showDictionaryInFolder = (dictName: string): void => {
        webSocket.value?.sendShowDictInFolder(dictName)
    }

    const syncDictConfigIfChanged = (): void => {
        if (JSON.stringify(localDictConfig.value) !== JSON.stringify(dictConfigStore.dictConfig)) {
            webSocket.value?.sendUpdateDictConfig(localDictConfig.value)
        }
    }

    const syncSessionConfigIfChanged = (): void => {
        if (JSON.stringify(localSessionConfig.value) !== JSON.stringify(sessionConfig.value)) {
            webSocket.value?.sendSessionConfig(localSessionConfig.value)
        }
    }

    watch(() => dictConfigStore.dictConfig, refresh, { deep: true })

    watch(currentOptionName, async () => {
        syncDictConfigIfChanged()
        updateList()
        await nextTick()
        initSortable()
    })

    return {
        list,
        currentOptionName,
        isDefaultOption,
        refresh,
        createOption,
        renameOption,
        deleteOption,
        deleteDictionary,
        showDictionaryInFolder,
        syncDictConfigIfChanged,
        syncSessionConfigIfChanged,
        destroySortable: () => sortableInstance?.destroy(),
    }
}