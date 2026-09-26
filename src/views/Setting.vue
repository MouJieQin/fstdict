<template>
    <div data-tauri-drag-region class="setting-layout">
        <el-container data-tauri-drag-region>
            <el-aside data-tauri-drag-region class="setting-sidebar" width="200px">
                <el-scrollbar>
                    <el-menu ref="menuRef" default-active="general" class="setting-menu">
                        <el-menu-item index="general" @click="handleItemClick('general')">
                            <template #title>
                                <el-icon>
                                    <Setting />
                                </el-icon>{{ t('settings.general') }}
                            </template>
                        </el-menu-item>

                        <el-menu-item index="shortcut" @click="handleItemClick('shortcut')">
                            <template #title>
                                <el-icon>
                                    <BsKeyboard />
                                </el-icon>{{ t('settings.shortcut') }}
                            </template>
                        </el-menu-item>

                        <el-menu-item index="wordLookup" @click="handleItemClick('wordLookup')">
                            <template #title>
                                <el-icon>
                                    <Message />
                                </el-icon>{{ t('settings.wordLookup') }}
                            </template>
                        </el-menu-item>
                    </el-menu>
                </el-scrollbar>
            </el-aside>
            <el-divider direction="vertical"
                style="height: 100vh; padding: 0;margin: 0; border: 1px solid var(--splitter-color);" />
            <el-main data-tauri-drag-region class="setting-main">
                <el-scrollbar class="scroll-container">
                    <General v-show="activeTabIndex === 'general'" :web-socket="webSocket" />
                    <Shortcut v-show="activeTabIndex === 'shortcut'" :web-socket="webSocket" />
                </el-scrollbar>
            </el-main>
        </el-container>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount, onMounted } from 'vue'
import type { MenuInstance, ElMenu } from 'element-plus'

import { Menu as IconMenu, Message, Setting } from '@element-plus/icons-vue'
import { BsKeyboard } from 'vue-icons-plus/bs'



// WebSocket & stores
import { useSessionWebSocket } from '@/common/session-websocket-client'
import {
    useFolderConfigStore,
    useDictConfigStore,
    useSystemConfigStore,
} from '@/stores'
import { getDefaultSessionConfig } from '@/common/utility'

// add import at top
import { setAppLocale } from '@/i18n'
import { useI18n } from 'vue-i18n'


// --- Components ---
import General from '@/components/Setting/General.vue'
import Shortcut from '@/components/Setting/Shortcut.vue'

const { t } = useI18n()

// --- Stores ---
const systemConfigStore = useSystemConfigStore()

// --- Reactive state ---
const webSocket = ref<ReturnType<typeof useSessionWebSocket> | null>(null)
const activeTabIndex = ref('general')
const menuRef = ref<MenuInstance>()



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
        case 'keyword_options_search':
            break
    }
}

// update handleSystemConfig function
const handleSystemConfig = (data: any): void => {
    systemConfigStore.setSystemConfig(data.system_config)
    // sync language preference
    const lang = data.system_config?.appearance?.language
    if (lang) setAppLocale(lang)
}

const handleItemClick = (index: string): void => {
    if (index === "wordLookup") {
        menuRef.value?.updateActiveIndex(activeTabIndex.value)
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
