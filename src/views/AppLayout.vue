<template>
    <div class="app-container" :class="{ 'is-main': envFromRoute === ENV.MAIN }">
        <el-container>
            <el-aside v-if="showSidebar" id="app-sidebar" class="app-sidebar"
                :class="{ 'is-collapsed': isMainSidebarCollapsed }" :width="isMainSidebarCollapsed ? '0px' : '200px'">
                <div class="common-layout">
                    <el-container style="height: 100vh">
                        <el-header data-tauri-drag-region :height="`var(--header-height)`"
                            style="display: flex; align-items: center; justify-content: flex-end;">
                            <el-button text style="" size="small"
                                @click="isMainSidebarCollapsed = !isMainSidebarCollapsed">
                                <el-icon size="20">
                                    <vsc-layout-sidebar-left-off />
                                </el-icon>
                            </el-button>
                        </el-header>
                        <el-main style="padding: 0;">
                            <el-scrollbar>
                                <el-menu ref="menuRef" :default-active="MAIN_MENU_INDEX.DICTIONARY"
                                    class="setting-menu">
                                    <el-menu-item :index="MAIN_MENU_INDEX.DICTIONARY"
                                        @click="handleItemClick(MAIN_MENU_INDEX.DICTIONARY)">
                                        <template #title>
                                            <el-icon>
                                                <Setting />
                                            </el-icon>{{ t('appLayout.dictionary') }}
                                        </template>
                                    </el-menu-item>

                                    <el-menu-item index="manage" @click="handleItemClick('manage')">
                                        <template #title>
                                            <el-icon>
                                                <Menu />
                                            </el-icon>
                                            {{ t('appLayout.manage') }}
                                        </template>
                                    </el-menu-item>

                                    <el-menu-item :index="MAIN_MENU_INDEX.HISTORY"
                                        @click="handleItemClick(MAIN_MENU_INDEX.HISTORY)">
                                        <template #title>
                                            <el-icon>
                                                <VscHistory />
                                            </el-icon>{{ t('appLayout.history') }}
                                        </template>
                                    </el-menu-item>

                                    <el-sub-menu :index="MAIN_MENU_INDEX.GLOSSARY" class="customized-sub-menu">
                                        <template #title>
                                            <el-icon>
                                                <PiFolderStar />
                                            </el-icon>
                                            <span>{{ t('appLayout.glossary') }}</span>
                                        </template>
                                        <div v-for="folder in folderConfigStore.folderConfig?.folders.folder_info"
                                            :key="folder.id">
                                            <el-menu-item
                                                :index="`${MAIN_MENU_INDEX.FLOSSARY_FOLDERS_PREFIX}${folder.id}`"
                                                @click="handleItemClick(`${MAIN_MENU_INDEX.FLOSSARY_FOLDERS_PREFIX}${folder.id}`)">
                                                <template #title>
                                                    <span>{{ folder.name }}</span>
                                                </template>
                                            </el-menu-item>
                                        </div>
                                    </el-sub-menu>
                                </el-menu>
                            </el-scrollbar>
                        </el-main>
                        <el-footer :height="`var(--header-height)`" class="footer">
                            <el-button text @click="handleSettingClick">
                                <el-icon>
                                    <Setting />
                                </el-icon>
                            </el-button>
                        </el-footer>
                    </el-container>
                </div>
            </el-aside>
            <el-divider v-if="showSidebar" direction="vertical"
                style="height: 100vh; padding: 0;margin: 0; border: 1px solid var(--splitter-color);" />
            <el-main style="padding:0">
                <DictPage :show-sidebar="showSidebar" :is-main-sidebar-collapsed="isMainSidebarCollapsed"
                    @toggle:main-sidebar="isMainSidebarCollapsed = $event" :active-menu-index="activeTabIndex"
                    :folder-name="folderName" :favorite-words="viewingFolderWords" />
            </el-main>
        </el-container>
    </div>
</template>


<script setup lang="ts">
// add import at top
import { setAppLocale } from '@/i18n'
import { useI18n } from 'vue-i18n'
import { ref, computed, watch, onMounted, onUnmounted, onBeforeUnmount, nextTick } from 'vue'
import type { MenuInstance, ElMenu } from 'element-plus'
import { useRouter, useRoute } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { platform } from '@tauri-apps/plugin-os'
import { isTauri, invoke } from '@tauri-apps/api/core'
import type { DictsInfo, FolderWords } from '@/common/type-interface'

import { ENV, TAURI_EVENT, MAIN_MENU_INDEX, TAURI_CMD } from '@/common/constants'

// WebSocket & stores
import { useSessionWebSocket } from '@/common/session-websocket-client'
import {
    useFolderConfigStore,
    useDictConfigStore,
    useSystemConfigStore,
} from '@/stores'

// Icons
import { VscLayoutSidebarLeftOff } from 'vue-icons-plus/vsc'
import { Setting, Menu } from '@element-plus/icons-vue'
import { PiFolderStar } from 'vue-icons-plus/pi'
import { VscHistory } from 'vue-icons-plus/vsc'


import DictPage from '@/views/DictPage.vue'

const { t } = useI18n()

// --- Stores ---
const systemConfigStore = useSystemConfigStore()
const dictConfigStore = useDictConfigStore()
const folderConfigStore = useFolderConfigStore()


// --- Router & route ---
const route = useRoute()

// --- Reactive state ---
const envFromRoute = ref('')
const webSocket = ref<ReturnType<typeof useSessionWebSocket> | null>(null)
const showSidebar = ref(false)
const isMainSidebarCollapsed = ref(false)
const redirectWord = ref('')
const menuRef = ref<MenuInstance>()
const activeTabIndex = ref('dictionary')
const folderWords = ref<FolderWords>({})
const viewingFolderId = ref<number>(0)


const headerPaddingRight = ref(0)
const headerPaddingLeft = ref(0)


// --- Computed properties ---
const viewingFolderWords = computed(() =>
    folderWords.value[viewingFolderId.value] || []
)

const folderName = computed(() => {
    const folder = folderConfigStore.folderConfig?.folders.folder_info.find(f => f.id === viewingFolderId.value)
    if (folder) {
        return folder.name
    }
    return ''
})

const initHeaderPaddingRight = () => {
    if (!isTauri()) {
        headerPaddingRight.value = 0
        showSidebar.value = true
        return
    } else {
        showSidebar.value = envFromRoute.value === ENV.MAIN
        if (platform() === 'macos') {
            headerPaddingRight.value = 0
        } else {
            if (envFromRoute.value === ENV.MAIN) {
                headerPaddingRight.value = 138
            }
        }
    }
}

const handleSettingClick = async (): Promise<void> => {
    await invoke(TAURI_CMD.SHOW_SETTING_WINDOW)
}

const handleItemClick = async (index: string): Promise<void> => {
    if (index === MAIN_MENU_INDEX.MANAGE) {
        menuRef.value?.updateActiveIndex(activeTabIndex.value)
        await invoke(TAURI_CMD.SHOW_MANAGE_WINDOW)
        return
    }
    if (index.startsWith(MAIN_MENU_INDEX.FLOSSARY_FOLDERS_PREFIX)) {
        viewingFolderId.value = Number(index.replace(MAIN_MENU_INDEX.FLOSSARY_FOLDERS_PREFIX, ''))
        webSocket.value?.sendFavoriteWordsRequest(viewingFolderId.value)
    }
    activeTabIndex.value = index
}

onMounted(async () => {
    envFromRoute.value = (route.query.env as string) || ENV.MAIN
    initHeaderPaddingRight()
    setupWebSocket()
})


watch(
    () => isMainSidebarCollapsed.value,
    (collapsed) => {
        if (!isTauri()) return
        if (showSidebar.value && platform() === 'macos' && envFromRoute.value === ENV.MAIN) {
            headerPaddingLeft.value = collapsed ? 100 : 0
        }
    }
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
        case 'favorite_words':
            folderWords.value[message.data.folder_id] = message.data.words
            break
    }
}

</script>

<style scoped>
.app-container {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
}

.main-content {
    flex: 1;
    overflow: hidden;
}
</style>