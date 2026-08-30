<template>
    <div class="center-icon-container">
        <img src="/icon.png" class="window-center-icon" />
    </div>
    <div class="permission-help">
        <!-- <el-button :icon="BiSolidHelpCircle" style="font-size: 30px;" type="primary" circle> -->
        <el-button :icon="BiHelpCircle" style="font-size:30px" circle size=small></el-button>
    </div>
    <div class="permission-card">
        <el-icon size="35">
            <Mouse />
        </el-icon>
        <div>
            <p class="permision-title">{{ $t('permission.accessibilityPermission') }}</p>
            <p class="permision-purpose">{{ $t('permission.accessibilityPurpose') }}</p>
        </div>
        <el-icon v-if="hasAccessibility" size="35" :color="`var(--el-color-success)`">
            <CircleCheckFilled />
        </el-icon>
        <el-button v-else type="primary" round @click="requestAccessibilitySafe" size="small">{{
            $t('permission.grantPermission')
            }}</el-button>
    </div>
    <div class="permission-card">
        <el-icon size="35">
            <BiScreenshot />
        </el-icon>
        <div>
            <p class="permision-title">{{ $t('permission.screenshotPermission') }}</p>
            <p class="permision-purpose">{{ $t('permission.screenshotPurpose') }}</p>
        </div>
        <el-icon v-if="hasAccessibility" size="35" :color="`var(--el-color-success)`">
            <CircleCheckFilled />
        </el-icon>
        <el-button v-else type="primary" round @click="requestAccessibilitySafe" size="small">{{
            $t('permission.grantPermission')
            }}</el-button>
    </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { Mouse, CircleCheckFilled } from '@element-plus/icons-vue'
import { BiScreenshot, BiSolidHelpCircle, BiHelpCircle } from 'vue-icons-plus/bi'
import { isMacOS, checkAccessibilitySafe, requestAccessibilitySafe, showPerssionWindow } from '@/common/permission'
import { CHECK_PERMISSION_INTERVAL } from '@/common/constants'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const checkAccessibilityTimer = ref<number | null>(null)
const hasAccessibility = ref(false)


onMounted(async () => {
    if (checkAccessibilityTimer.value) {
        clearInterval(checkAccessibilityTimer.value)
    }
    checkAccessibilityTimer.value = setInterval(async () => {
        hasAccessibility.value = await checkAccessibilitySafe()
    }, CHECK_PERMISSION_INTERVAL)
})

onUnmounted(() => {
    if (checkAccessibilityTimer.value) {
        clearInterval(checkAccessibilityTimer.value)
    }
})

</script>
