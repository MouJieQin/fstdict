<template>
    <div class="anki-progress">
        <el-progress :percentage="percentage" :status="status" :indeterminate="indeterminate" :duration="5">
            <span v-if="showInfo">{{ info }}</span>
        </el-progress>
        <div class="anki-update-progress">
            <p>Total words: {{ props.ankiProgress?.data?.total_count || 0 }}</p>
            <p>Total handled words: {{ props.ankiProgress?.data?.count || 0 }}</p>
            <p>Successfully updated words: {{ props.ankiProgress?.data?.updated_count || 0 }}</p>
            <p>Successfully created words: {{ props.ankiProgress?.data?.created_count || 0 }}</p>
            <p>Failed to update words: {{ props.ankiProgress?.data?.update_error_count || 0 }}</p>
            <p>Failed to create words: {{ props.ankiProgress?.data?.create_error_count || 0 }}</p>
            <p v-if="errorMessage" style="color: red;">{{ errorMessage }}</p>
        </div>
    </div>

</template>

<script lang="ts" setup>
import { SessionWebSocketService } from '@/common/session-websocket-client'
import { watch, computed, ref } from 'vue'


const status = ref('success')
const indeterminate = ref(true)
const showInfo = ref(true)
const errorMessage = ref('')
const info = ref('')

const props = defineProps({
    webSocket: {
        type: [SessionWebSocketService, null],
        required: true
    },
    ankiProgress: {
        type: Object,
        default: () => ({})
    },
    ankiDialogVisible: {
        type: Boolean,
        required: true,
        default: false
    }
})

const initValue = () => {
    status.value = 'success'
    indeterminate.value = true
    showInfo.value = true
    errorMessage.value = ''
}

watch(() => props.ankiDialogVisible, (newVal) => {
    if (!newVal) {
        initValue()
    }
})

const percentage = computed(() => {
    return Math.floor(100 * props.ankiProgress?.data?.count / props.ankiProgress?.data?.total_count) || 30
})

watch(() => props.ankiProgress?.data?.count, (newVal) => {
    info.value = `(${newVal}/${props.ankiProgress?.data?.total_count || 0})`
})



watch(() => props.ankiProgress?.type, (newVal) => {
    errorMessage.value = ''
    try {
        if (newVal === "trying_acquiring_cards_from_anki") {
            status.value = 'success'
            indeterminate.value = true
            showInfo.value = true
            info.value = 'Trying acquiring cards from Anki...'
        }
        else if (newVal === "updating_cards") {
            status.value = 'success'
            indeterminate.value = false
            showInfo.value = true
        }
        else if (newVal === "done") {
            status.value = 'success'
            indeterminate.value = false
            showInfo.value = false
            info.value = ''
        }
        else if (newVal === "error") {
            status.value = 'exception'
            indeterminate.value = false
            showInfo.value = false
            info.value = ''
            errorMessage.value = props.ankiProgress?.data?.error_message || 'Unknown error'
        }
        else {
        }
    } catch (error) {
        console.error(error)
    }
})

</script>

<style scoped>
.anki-progress .el-progress--line {
    margin-bottom: 15px;
    max-width: 600px;
}
</style>