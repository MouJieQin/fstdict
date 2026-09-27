<template>
    <div class="app-container" :class="{ 'is-main': envFromRoute === ENV.MAIN }">
        <el-container>
            <el-aside id="app-sidebar" v-if="showSidebar" class="app-sidebar"
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
                            <el-scrollbar class="sidebar-inner">
                                <el-menu :default-openeds="['1', '3']" class="main-menu">
                                    <el-sub-menu index="1">
                                        <template #title>
                                            <el-icon>
                                                <message />
                                            </el-icon>Navigator One
                                        </template>
                                        <el-menu-item-group>
                                            <template #title>Group 1</template>
                                            <el-menu-item index="1-1">Option 1</el-menu-item>
                                            <el-menu-item index="1-2">Option 2</el-menu-item>
                                        </el-menu-item-group>
                                        <el-menu-item-group title="Group 2">
                                            <el-menu-item index="1-3">Option 3</el-menu-item>
                                        </el-menu-item-group>
                                        <el-sub-menu index="1-4">
                                            <template #title>Option4</template>
                                            <el-menu-item index="1-4-1">Option 4-1</el-menu-item>
                                        </el-sub-menu>
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
                    @toggle:main-sidebar="isMainSidebarCollapsed = $event" />
                <!-- <el-container>
                <el-header data-tauri-drag-region :height="`var(--header-height)`" id="fstdict-header"
                    class="fstdict-header" :style="{
                        '--header-padding-right': `${headerPaddingRight}px`,
                        '--header-padding-left': `${headerPaddingLeft}px`
                    }">
                </el-header>
                <div class="main-content">
                    <router-view />
                </div>
            </el-container> -->
            </el-main>
        </el-container>
    </div>
</template>


<script setup lang="ts">
// add import at top
import { setAppLocale } from '@/i18n'
import { ref, computed, watch, onMounted, onUnmounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { platform } from '@tauri-apps/plugin-os'
import { isTauri, invoke } from '@tauri-apps/api/core'

import { ENV, TAURI_EVENT, TAURI_CMD } from '@/common/constants'

// WebSocket & stores
import { useSessionWebSocket } from '@/common/session-websocket-client'



// Icons
import { VscLayoutSidebarLeftOff } from 'vue-icons-plus/vsc'
import { Setting } from '@element-plus/icons-vue'

import DictPage from '@/views/DictPage.vue'


// --- Router & route ---
const route = useRoute()

// --- Reactive state ---
const envFromRoute = ref('')
const webSocket = ref<ReturnType<typeof useSessionWebSocket> | null>(null)
const showSidebar = ref(false)
const isMainSidebarCollapsed = ref(false)
const redirectWord = ref('')

const headerPaddingRight = ref(0)
const headerPaddingLeft = ref(0)


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

onMounted(async () => {
    envFromRoute.value = (route.query.env as string) || ENV.MAIN
    initHeaderPaddingRight()
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