<template>
    <svg :width="width" :height="height" viewBox="0 0 120 30" xmlns="http://www.w3.org/2000/svg" :fill="color"
        :aria-label="$t('common.loading')" role="img">
        <circle cx="15" cy="15" r="15">
            <animate attributeName="r" values="15;9;15" dur="0.8s" calcMode="linear" repeatCount="indefinite" />
            <animate attributeName="fill-opacity" values="1;.5;1" dur="0.8s" calcMode="linear"
                repeatCount="indefinite" />
        </circle>
        <circle cx="60" cy="15" r="9" fill-opacity="0.3">
            <animate attributeName="r" values="9;15;9" dur="0.8s" calcMode="linear" repeatCount="indefinite" />
            <animate attributeName="fill-opacity" values=".5;1;.5" dur="0.8s" calcMode="linear"
                repeatCount="indefinite" />
        </circle>
        <circle cx="105" cy="15" r="15">
            <animate attributeName="r" values="15;9;15" dur="0.8s" calcMode="linear" repeatCount="indefinite" />
            <animate attributeName="fill-opacity" values="1;.5;1" dur="0.8s" calcMode="linear"
                repeatCount="indefinite" />
        </circle>
    </svg>
</template>

<script setup lang="ts">
interface Props {
    width?: number | string
    height?: number | string
    color?: string
}

withDefaults(defineProps<Props>(), {
    width: 20,
    height: 5,
    color: 'var(--puff-loader-color)',
})
</script>