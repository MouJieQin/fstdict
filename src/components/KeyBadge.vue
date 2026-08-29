<template>
    <span class="key-badge">{{ text }}</span>
</template>

<script setup lang="ts">
/**
 * Single key cap badge component
 * VSCode-style rounded key with bottom border depth
 */
defineProps<{
    text: string
}>()
</script>

<style scoped>
.key-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    padding: 0 6px;
    margin: 0 2px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    line-height: 1;
    color: var(--el-text-color-primary);
    background: var(--el-bg-color-page);
    border: 1px solid var(--el-border-color);
    border-bottom-width: 2px;
    border-radius: 4px;
    user-select: none;
}

.dark .key-badge {
    background: var(--el-bg-color);
    border-color: var(--el-border-color-darker);
}
</style>
