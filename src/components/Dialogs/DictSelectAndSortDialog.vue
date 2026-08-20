<template>
    <div>
        <!-- Drag-and-drop area (Tauri only) -->
        <div v-if="isTauriEnv" class="drag-area" :class="{ active: dragOver }">
            <BsUpload size="35" />
            <div>{{ $t('dictDialog.dragHint') }}</div>
            <div>
                {{ $t('dictDialog.visitForum') }}
                <el-link href="https://forum.freemdict.com" type="primary" target="_blank">
                    FreeMdict Forum
                </el-link>
                {{ $t('dictDialog.orDownloadFrom') }}
                <el-link href="https://downloads.freemdict.com" type="primary" target="_blank">
                    downloads
                </el-link>
            </div>
        </div>

        <!-- Controls -->
        <div class="dict-set-options">
            <div class="dict-set-options-control">
                <div style="text-align: center">
                    <el-button type="primary" :icon="Plus" @click="createOption" />
                    <el-button type="danger" :icon="Delete" @click="deleteOption" :disabled="isDefaultOption" />
                    <el-button :icon="Edit" @click="renameOption" :disabled="isDefaultOption" />

                    <el-select v-model="currentOptionName" filterable :placeholder="$t('dictSettings.selectPreset')"
                        style="margin-left: 20px; max-width: 240px">
                        <el-option v-for="(_, name) in dictSetOptions" :key="name" :label="name" :value="name" />
                    </el-select>
                </div>
            </div>

            <!-- Sortable card list -->
            <div ref="listRef" class="dict-select-sort-dialog">
                <div v-for="item in list" :key="item.name" class="dict-settings-drag-cards">
                    <el-card class="dict-settings-drag-card" shadow="always"
                        :class="{ 'is-disabled': !item.is_enabled }">
                        <div class="dict-settings-drag-card-content">
                            <div class="left-group">
                                <el-image :src="dictsInfo[item.name]?.cover_url" class="icon">
                                    <template #error>
                                        <BiSolidBookBookmark size="35" />
                                    </template>
                                </el-image>
                                <span class="name">{{ item.name }}</span>
                            </div>
                            <div class="right-group">
                                <el-switch v-model="item.is_enabled" style="margin-right: 30px" />
                                <el-dropdown placement="bottom-end" @command="handleCardCommand">
                                    <el-icon style="align-items: center">
                                        <MoreFilled />
                                    </el-icon>
                                    <template #dropdown>
                                        <el-dropdown-menu>
                                            <el-dropdown-item :command="{ cmd: 'showInFolder', name: item.name }">
                                                {{ $t('dictDialog.showInFolder') }}
                                            </el-dropdown-item>
                                            <el-dropdown-item :command="{ cmd: 'delete', name: item.name }">
                                                <el-icon>
                                                    <Delete style="color: #ff4949" />
                                                </el-icon>
                                                <span>{{ $t('dictDialog.deleteDict') }}</span>
                                            </el-dropdown-item>
                                        </el-dropdown-menu>
                                    </template>
                                </el-dropdown>
                            </div>
                        </div>
                    </el-card>
                </div>
            </div>
        </div>

        <!-- Add dictionary progress dialog -->
        <el-dialog v-model="addDictVisible" :title="$t('dictDialog.addingDict')" width="700"
            :close-on-click-modal="false" :close-on-press-escape="false" draggable :show-close="false">
            <div v-for="(msg, index) in addDictMsgs" :key="index" class="add-dict-message">
                <p v-if="msg.type === 'info'" class="msg-info">{{ msg.msg }}</p>
                <p v-else-if="msg.type === 'warning'" class="msg-warning">{{ msg.msg }}</p>
                <p v-else-if="msg.type === 'error'" class="msg-error">{{ msg.msg }}</p>
                <p v-else-if="msg.type === 'success'" class="msg-success">{{ msg.msg }}</p>
                <p v-else-if="msg.type === 'done'" class="msg-success">{{ $t('common.done') }}</p>
            </div>

            <template #footer>
                <div class="dialog-footer">
                    <el-button v-if="isAddDictDone" type="primary" @click="addDictVisible = false">
                        {{ $t('common.close') }}
                    </el-button>
                </div>
            </template>
        </el-dialog>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import type { PropType } from 'vue'
import { MoreFilled, Delete, Edit, Plus } from '@element-plus/icons-vue'
import { BiSolidBookBookmark } from 'vue-icons-plus/bi'
import { BsUpload } from 'vue-icons-plus/bs'
import { getCurrentWebview } from '@tauri-apps/api/webview'

import { useDictSetOptions } from '@/composables/useDictSetOptions'
import type { SessionConfig } from '@/common/type-interface'
import { SessionWebSocketService } from '@/common/session-websocket-client'
import { useDictConfigStore } from '@/stores/dictConfig'

const props = defineProps({
    dictSSDialogVisible: {
        type: Boolean,
        required: true,
    },
    env: {
        type: String,
        default: 'web',
    },
    webSocket: {
        type: [SessionWebSocketService, null],
        required: true,
    },
    sessionConfig: {
        type: Object as PropType<SessionConfig>,
        required: true,
    },
    dictsInfo: {
        type: Object as PropType<Record<string, any>>,
        required: true,
    },
    addDictMsgs: {
        type: Array,
        default: () => [],
    },
    refreshDicsSettingsInfoFlag: {
        type: Boolean,
        default: true,
    },
})

const emit = defineEmits<{
    (e: 'clear:addDictMsgs'): void
}>()

// --- Refs ---
const listRef = ref<HTMLElement | null>(null)
const addDictVisible = ref(false)
const dragOver = ref(false)

let unlistenDragDrop: (() => void) | null = null

// --- Computed ---
const isTauriEnv = computed(() => props.env === '')

const isAddDictDone = computed(() => {
    const msgs = props.addDictMsgs
    return msgs.length > 0 && msgs[msgs.length - 1].type === 'done'
})

// --- Dict set options composable ---
const wsRef = computed(() => props.webSocket)
const configRef = computed(() => props.sessionConfig)

const {
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
    destroySortable,
} = useDictSetOptions({
    webSocket: wsRef,
    sessionConfig: configRef,
    listContainerRef: listRef,
})

// Expose dict_set_options for select dropdown
const dictSetOptions = computed(() => {
    return useDictConfigStore().dictConfig?.dict_set_options || {}
})

// --- Card dropdown commands ---
const handleCardCommand = (command: { cmd: string; name: string }): void => {
    switch (command.cmd) {
        case 'showInFolder':
            showDictionaryInFolder(command.name)
            break
        case 'delete':
            deleteDictionary(command.name)
            break
    }
}

// --- File drag-and-drop (Tauri) ---
const processDroppedFiles = (paths: string[]): void => {
    emit('clear:addDictMsgs')
    addDictVisible.value = true
    for (const filePath of paths) {
        props.webSocket?.sendAddDictionary(filePath)
    }
}

const setupDragAndDrop = async (): Promise<void> => {
    if (props.env !== '') return

    const webview = getCurrentWebview()

    unlistenDragDrop = await webview.onDragDropEvent((event) => {
        const payload = event.payload

        switch (payload.type) {
            case 'enter':
            case 'over': {
                const dragArea = document.querySelector('.drag-area')?.getBoundingClientRect()
                if (dragArea && payload.position) {
                    const { x, y } = payload.position
                    dragOver.value =
                        x >= dragArea.left &&
                        x <= dragArea.right &&
                        y >= dragArea.top &&
                        y <= dragArea.bottom
                }
                break
            }
            case 'drop': {
                if (dragOver.value && payload.paths) {
                    processDroppedFiles(payload.paths)
                }
                dragOver.value = false
                break
            }
            case 'leave':
            default:
                dragOver.value = false
                break
        }
    })
}

// --- Dialog lifecycle ---
watch(
    () => props.dictSSDialogVisible,
    async (visible) => {
        if (visible) {
            await refresh()
        } else {
            syncDictConfigIfChanged()
            syncSessionConfigIfChanged()
        }
    },
    { deep: true }
)

watch(() => props.refreshDicsSettingsInfoFlag, () => {
    refresh()
})

onMounted(async () => {
    if (props.dictSSDialogVisible) {
        await nextTick()
        refresh()
    }
    await setupDragAndDrop()
})

onBeforeUnmount(() => {
    destroySortable()
    if (unlistenDragDrop) {
        unlistenDragDrop()
    }
})
</script>

<style scoped>
.dialog-footer {
    display: flex;
    justify-content: flex-end;
}

.add-dict-message p {
    margin: 4px 0;
}

.msg-info {
    color: var(--el-color-primary);
}

.msg-warning {
    color: var(--el-color-warning);
}

.msg-error {
    color: var(--el-color-danger);
}

.msg-success {
    color: var(--el-color-success);
}
</style>