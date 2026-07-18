<template>
  <main id="app" class="flex-grow container mx-auto px-4 py-8">
    <router-view id="app" />
  </main>
</template>

<script lang="ts" setup>
import { onMounted } from 'vue'
import { useTheme } from '@/common/use-theme';

const { initTheme, watchSystemTheme } = useTheme();

onMounted(() => {
  initTheme();
  watchSystemTheme();
});
</script>