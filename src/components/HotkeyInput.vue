<template>
    <div class="hotkey-input-wrapper">
        <div class="hotkey-input" :class="{ capturing, 'is-error': hasConflict }" tabindex="0" @click="startCapture"
            @keydown="handleKeyDown" @blur="cancelCapture">
            <template v-if="capturing">
                <span class="capture-hint">{{ t('settings.pressKeys') }}</span>
            </template>
            <template v-else-if="displayKeys.length > 0">
                <KeyBadge v-for="(key, idx) in displayKeys" :key="idx" :text="key" />
            </template>
            <template v-else>
                <span class="placeholder">{{ t('settings.clickToSet') }}</span>
            </template>
        </div>

        <el-button v-if="modelValue.length > 0" size="small" text @click.stop="clearHotkey"
            :aria-label="t('common.clear')">
            {{ t('common.clear') }}
        </el-button>
    </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import KeyBadge from './KeyBadge.vue'
import {
    normalizeKeyFromCode,
    isModifier,
    sortHotkey,
    formatHotkey,
    type NormalizedKey,
} from '@/common/hotkey'

/**
 * Interactive hotkey capture input
 * - Click to enter capture mode
 * - Press key combination to record
 * - Press ESC to cancel
 * - Blur to cancel
 * ✅ Fixed: Use KeyboardEvent.code for physical key, avoid Mac Option special char issue
 */
const props = defineProps<{
    modelValue: NormalizedKey[]
    conflictCheck?: (keys: NormalizedKey[]) => boolean
}>()

const emit = defineEmits<{
    (e: 'update:modelValue', value: NormalizedKey[]): void
}>()

const { t } = useI18n()

const capturing = ref(false)
const tempKeys = ref<NormalizedKey[]>([])

// Platform-formatted display keys
const displayKeys = computed(() => formatHotkey(props.modelValue))

// Conflict validation state
const hasConflict = computed(() => {
    if (!props.conflictCheck) return false
    return props.conflictCheck(props.modelValue)
})

/**
 * Enter capture mode
 */
function startCapture(): void {
    if (capturing.value) return
    capturing.value = true
    tempKeys.value = []
}

/**
 * Handle keyboard input during capture
 */
function handleKeyDown(e: KeyboardEvent): void {
    if (!capturing.value) return

    e.preventDefault()
    e.stopPropagation()

    // ESC cancels capture
    if (e.key === 'Escape') {
        cancelCapture()
        return
    }

    // Build modifier set from event flags
    const activeModifiers: NormalizedKey[] = []
    if (e.metaKey) activeModifiers.push('meta')
    if (e.ctrlKey) activeModifiers.push('ctrl')
    if (e.altKey) activeModifiers.push('alt')
    if (e.shiftKey) activeModifiers.push('shift')

    const physicalKey = normalizeKeyFromCode(e.code)
    const activeKeys: NormalizedKey[] = [...activeModifiers]

    if (physicalKey && !isModifier(physicalKey)) {
        activeKeys.push(physicalKey)
    }

    tempKeys.value = sortHotkey(activeKeys)

    // Commit when we have at least one non-modifier key
    if (physicalKey && !isModifier(physicalKey)) {
        commitCapture()
    }
}

/**
 * Save captured keys to model
 */
function commitCapture(): void {
    if (tempKeys.value.some((k) => !isModifier(k))) {
        emit('update:modelValue', [...tempKeys.value])
    }
    cancelCapture()
}

/**
 * Exit capture mode
 */
function cancelCapture(): void {
    capturing.value = false
    tempKeys.value = []
}

/**
 * Clear current hotkey
 */
function clearHotkey(): void {
    emit('update:modelValue', [])
}
</script>

<style scoped>
.hotkey-input-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
}

.hotkey-input {
    display: flex;
    align-items: center;
    min-width: 220px;
    min-height: 32px;
    padding: 4px 8px;
    background: var(--el-input-bg-color);
    border: 1px solid var(--el-input-border-color);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s;
}

.hotkey-input:hover {
    border-color: var(--el-color-primary);
}

.hotkey-input.capturing {
    border-color: var(--el-color-primary);
    box-shadow: 0 0 0 2px rgba(var(--el-color-primary-rgb), 0.15);
}

.hotkey-input.is-error {
    border-color: var(--el-color-danger);
}

.capture-hint {
    font-size: 13px;
    color: var(--el-color-primary);
}

.placeholder {
    font-size: 13px;
    color: var(--el-text-color-placeholder);
}
</style>
