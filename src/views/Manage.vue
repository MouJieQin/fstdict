<template>
    <div data-tauri-drag-region class="setting-layout">
        <el-container data-tauri-drag-region>
            <el-aside data-tauri-drag-region class="setting-sidebar" width="200px">
                <el-scrollbar>
                    <el-menu ref="menuRef" default-active="general" class="setting-menu">
                        <el-menu-item index="general" @click="handleItemClick('general')">
                            <template #title>
                                <el-icon>
                                    <PiBooks />
                                </el-icon>{{ t('manage.installedDicts') }}
                            </template>
                        </el-menu-item>

                        <el-menu-item index="glossary" @click="handleItemClick('glossary')">
                            <template #title>
                                <el-icon>
                                    <Star />
                                </el-icon>{{ t('manage.glossary') }}
                            </template>
                        </el-menu-item>

                        <el-sub-menu index="GlossaryFolders" class="customized-sub-menu">
                            <template #title>
                                <el-icon>
                                    <PiFolderStar />
                                </el-icon>
                                <span>{{ t('manage.glossaryFolders') }}</span>
                            </template>
                            <div v-for="folder in folderConfigStore.folderConfig?.folders.folder_info" :key="folder.id">
                                <el-menu-item :index="`GlossaryFolders-${folder.id}`"
                                    @click="handleItemClick(`GlossaryFolders-${folder.id}`)">
                                    <template #title>
                                        <span>{{ folder.name }}</span>
                                    </template>
                                </el-menu-item>
                            </div>
                        </el-sub-menu>

                    </el-menu>
                </el-scrollbar>
            </el-aside>
            <el-divider direction="vertical"
                style="height: 100vh; padding: 0;margin: 0; border: 1px solid var(--splitter-color);" />
            <el-main data-tauri-drag-region class="setting-main">
                <el-scrollbar class="scroll-container">
                    <InstalledDictionary v-show="activeTabIndex === 'general'" :web-socket="webSocket"
                        :dicts-info="dictsInfo" :add-dict-msgs="addDictMsgs" @clear:add-dict-msgs="addDictMsgs = []" />
                    <GlossaryOverview v-show="activeTabIndex === 'glossary'" :web-socket="webSocket"
                        :folder-words="folderWords" :anki-progresses="ankiProgresses" />
                    <FavoriteWords v-show="showFavoriteWords" :web-socket="webSocket" :folder-id="viewingFolderId"
                        :folderName="folderName" :favorite-words="viewingFolderWords" />
                </el-scrollbar>
            </el-main>
        </el-container>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount, onMounted } from 'vue'
import type { MenuInstance, ElMenu } from 'element-plus'

import { PiBooks } from 'vue-icons-plus/pi'
import { Star } from '@element-plus/icons-vue'
import { PiFolderStar } from 'vue-icons-plus/pi'



import type { DictsInfo, FolderWords } from '@/common/type-interface'


// WebSocket & stores
import { useSessionWebSocket } from '@/common/session-websocket-client'
import {
    useFolderConfigStore,
    useDictConfigStore,
    useSystemConfigStore,
} from '@/stores'
import { getDefaultSessionConfig } from '@/common/utility'

// add import at top
import { useI18n } from 'vue-i18n'


// --- Components ---
import InstalledDictionary from '@/components/Manage/InstalledDictionary.vue'
import GlossaryOverview from '@/components/Manage/GlossaryOverview.vue'
import FavoriteWords from '@/components/Manage/FavoriteWords.vue'


const { t } = useI18n()

// --- Stores ---
const systemConfigStore = useSystemConfigStore()
const dictConfigStore = useDictConfigStore()
const folderConfigStore = useFolderConfigStore()


// --- Reactive state ---
const webSocket = ref<ReturnType<typeof useSessionWebSocket> | null>(null)
const activeTabIndex = ref('general')
const menuRef = ref<MenuInstance>()
const dictsInfo = ref<DictsInfo>({})
const addDictMsgs = ref<any[]>([])
const ankiProgresses = ref<Record<string, any>>({})
const viewingFolderId = ref<number>(0)
const folderWords = ref<FolderWords>({})

// --- Computed properties ---
const showFavoriteWords = computed(() => {
    return activeTabIndex.value.startsWith('GlossaryFolders-')
})

const folderName = computed(() => {
    const folder = folderConfigStore.folderConfig?.folders.folder_info.find(f => f.id === viewingFolderId.value)
    if (folder) {
        return folder.name
    }
    return ''
})

const viewingFolderWords = computed(() =>
    folderWords.value[viewingFolderId.value] || []
)

// --- WebSocket setup ---
const setupWebSocket = (): void => {
    webSocket.value = useSessionWebSocket(0)

    if (webSocket.value) {
        webSocket.value.setMessageHandler(handleWebSocketMessage as any)
    }
}

const handleWebSocketMessage = async (message: any): Promise<void> => {
    switch (message.type) {
        case 'system_config':
            handleSystemConfig(message.data)
            break
        case 'dict_config':
            dictConfigStore.setDictConfig(message.data.dict_config)
            break
        case 'dict_info':
            dictsInfo.value = message.data
            break
        case 'add_dictionary':
            addDictMsgs.value.push(message.data)
            break
        case 'folder_config':
            folderConfigStore.setFolderConfig(message.data)
            break
        case 'anki_progress':
            ankiProgresses.value[message.deck_name] = message.data
            break
        case 'favorite_words':
            folderWords.value[message.data.folder_id] = message.data.words
            break
    }
}

// update handleSystemConfig function
const handleSystemConfig = (data: any): void => {
    systemConfigStore.setSystemConfig(data.system_config)
}

const handleItemClick = (index: string): void => {
    if (index === "wordLookup") {
        menuRef.value?.updateActiveIndex(activeTabIndex.value)
    } else if (index.startsWith("GlossaryFolders-")) {
        viewingFolderId.value = Number(index.split("-")[1])
        webSocket.value?.sendFavoriteWordsRequest(viewingFolderId.value)
        activeTabIndex.value = index

    } else {
        activeTabIndex.value = index
    }
}

// --- Lifecycle ---
const initDictPage = async (): Promise<void> => {
    setupWebSocket()
}

onMounted(async () => {
    await initDictPage()
})

onBeforeUnmount(() => {
    webSocket.value?.close()
})

</script>
