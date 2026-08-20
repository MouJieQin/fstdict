<template>
    <div class="anki-progress">
        <el-progress :percentage="percentage" :status="status" :indeterminate="indeterminate" :duration="5">
            <span v-if="showInfo">{{ infoText }}</span>
        </el-progress>

        <div class="anki-update-progress">
            <p>{{ $t('anki.totalWords') }}{{ ankiProgress?.data?.total_count || 0 }}</p>
            <p>{{ $t('anki.totalHandled') }}{{ ankiProgress?.data?.count || 0 }}</p>
            <p>{{ $t('anki.successUpdated') }}{{ ankiProgress?.data?.updated_count || 0 }}</p>
            <p>{{ $t('anki.successCreated') }}{{ ankiProgress?.data?.created_count || 0 }}</p>
            <p>{{ $t('anki.failedUpdate') }}{{ ankiProgress?.data?.update_error_count || 0 }}</p>
            <p>{{ $t('anki.failedCreate') }}{{ ankiProgress?.data?.create_error_count || 0 }}</p>
            <p v-if="errorMessage" class="error-text">{{ errorMessage }}</p>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { ref, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SessionWebSocketService } from '@/common/session-websocket-client'

interface AnkiProgressProps {
    webSocket: SessionWebSocketService | null
    ankiProgress: any
    ankiDialogVisible: boolean
}

const props = defineProps<AnkiProgressProps>()
const { t } = useI18n()

const status = ref<'success' | 'exception' | 'warning'>('success')
const indeterminate = ref(true)
const showInfo = ref(true)
const errorMessage = ref('')
const infoText = ref('')

const percentage = computed(() => {
    const total = props.ankiProgress?.data?.total_count
    const count = props.ankiProgress?.data?.count
    if (!total || total === 0) return 30
    return Math.floor((100 * count) / total)
})

function resetState(): void {
    status.value = 'success'
    indeterminate.value = true
    showInfo.value = true
    errorMessage.value = ''
    infoText.value = t('anki.waiting')
}

watch(() => props.ankiDialogVisible, (visible) => {
    if (!visible) resetState()
})

watch(() => props.ankiProgress?.data?.count, () => {
    infoText.value = `${percentage.value}%`
})

watch(() => props.ankiProgress?.type, (type) => {
    errorMessage.value = ''

    switch (type) {
        case 'trying_acquiring_cards_from_anki':
            status.value = 'success'
            indeterminate.value = true
            showInfo.value = true
            infoText.value = t('anki.fetchingCards')
            break

        case 'progress':
            status.value = 'success'
            indeterminate.value = false
            showInfo.value = true
            break

        case 'done':
            status.value = 'success'
            indeterminate.value = false
            showInfo.value = false
            infoText.value = ''
            break

        case 'error':
            status.value = 'exception'
            indeterminate.value = false
            showInfo.value = false
            infoText.value = ''
            errorMessage.value = props.ankiProgress?.data?.error_message || t('common.unknownError')
            break

        case 'canceled':
            status.value = 'warning'
            indeterminate.value = false
            showInfo.value = false
            infoText.value = ''
            break
    }
})
</script>

<style scoped>
.anki-progress .el-progress--line {
    margin-bottom: 15px;
    max-width: 600px;
}

.error-text {
    color: var(--el-color-danger);
}
</style>