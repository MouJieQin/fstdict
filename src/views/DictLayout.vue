<template>
    <div class="app-container">
        <div class="main-content">
            <router-view />
        </div>
    </div>
</template>

<script setup lang="ts">
// Root layout wrapper for all dictionary application views.
// Sidebar can be added here in the future.
</script>

<style scoped>
.app-container {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
}

.main-content {
    flex: 1;
    overflow: hidden;
}
</style>