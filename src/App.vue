<template>
    <el-config-provider :locale="elementPlusLocale">
        <main id="app" class="flex-grow container mx-auto px-4 py-8">
            <router-view />
        </main>
    </el-config-provider>
</template>

<script lang="ts" setup>
import { onMounted } from 'vue'
import { useTheme } from '@/common/use-theme'
import { elementPlusLocale } from '@/i18n'

const { initTheme, watchSystemTheme } = useTheme()

onMounted(() => {
    initTheme()
    watchSystemTheme()
})
</script>