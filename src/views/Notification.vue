<template>
    <div id="container" class="wrapper" :class="{ 'fade-out': isFadeOut }">
        <img src="/icon.png" class="icon-container" alt="icon" />
        <div id="text" class="message">{{ message }}</div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { useRoute } from 'vue-router'
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';

// reactive toast message content
const message = ref('');
const isFadeOut = ref(false);
const route = useRoute()

// store event unsubscribe functions
let unlistenFadeOut: UnlistenFn | null = null;
let unlistenUpdateMsg: UnlistenFn | null = null;

/**
 * Read initial message from URL search params
 */
function readUrlParams() {
    message.value = (route.query.message as string) || ''
}

/**
 * Register Tauri backend event listeners
 */
async function registerTauriEventListeners() {
    // listen for fade out trigger from backend
    unlistenFadeOut = await listen('start-fade-out', () => {
        console.log("Fade event received, applying CSS fade class.");
        isFadeOut.value = true;
    });

    // listen for message update from backend
    unlistenUpdateMsg = await listen('update-message', (event) => {
        console.log("Update event received. New text:", event.payload);
        message.value = event.payload as string;
        // reset fade state
        isFadeOut.value = false;
    });
}

onMounted(async () => {
    readUrlParams();
    await registerTauriEventListeners();
});

onUnmounted(() => {
    // clean up tauri event listeners to avoid memory leak
    if (unlistenFadeOut) unlistenFadeOut();
    if (unlistenUpdateMsg) unlistenUpdateMsg();
});
</script>

<style scoped>
html,
body {
    margin: 0;
    padding: 0;
    height: 100%;
    background: transparent !important;
    overflow: hidden;
}

.wrapper {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 10px;
    background: rgba(30, 30, 30, 0.85);
    backdrop-filter: blur(25px);
    -webkit-backdrop-filter: blur(25px);
    user-select: none;
    -webkit-user-select: none;
    color: white;
    font-family: -apple-system, BlinkMacSystemFont, sans-serif;
    border-radius: 14px;
    display: flex;
    align-items: center;
    gap: 14px;
    border: 0.5px solid rgba(255, 255, 255, 0.18);

    opacity: 1;
    transition: opacity 500ms cubic-bezier(0.4, 0, 0.2, 1);
}

.wrapper.fade-out {
    opacity: 0;
}

.icon-container {
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}

.message {
    font-size: 20px;
    font-weight: 500;
    line-height: 1.4;
    word-break: break-word;
}
</style>
