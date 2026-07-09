<template>
    <div class="icon-select-wrapper" @click.stop="openSelect">
        <!-- 当前选中的图标 -->
        <component :is="currentIcon" class="prefix-icon" />

        <!-- 下拉框（隐藏原框，只保留功能） -->
        <el-select v-model="selectedType" ref="selectRef" class="hidden-select" placeholder="">
            <el-option label="前缀搜索" value="prefix_search">
                <template #default>
                    <BsSearch class="option-icon" />
                    <span>前缀搜索</span>
                </template>
            </el-option>

            <el-option label="正则搜索" value="regex_search">
                <template #default>
                    <BsFilterLeft class="option-icon" />
                    <span>正则搜索</span>
                </template>
            </el-option>

            <el-option label="前缀距离搜索" value="prefix_distance_search">
                <template #default>
                    <BsBinoculars class="option-icon" />
                    <span>前缀距离搜索</span>
                </template>
            </el-option>

            <el-option label="建议搜索" value="suggest_search">
                <template #default>
                    <BsSearchHeart class="option-icon" />
                    <span>建议搜索</span>
                </template>
            </el-option>
        </el-select>
    </div>
</template>


<script lang="ts" setup>
import { ref, computed, watch } from 'vue'

import {
    BsSearch,
    BsFilterLeft,
    BsBinoculars,
    BsSearchHeart
} from 'vue-icons-plus/bs'

const props = defineProps({
    searchMethod: {
        type: String,
        default: 'prefix_search', // 默认搜索方法
    }
})

const emits = defineEmits<{
    (e: 'update-search-method', searchMethod: string): void
}>()

// 下拉选中值
const selectedType = ref(props.searchMethod) // 默认值

// 下拉框实例（用于控制弹出）
import type { ElSelect } from 'element-plus'
const selectRef = ref<InstanceType<typeof ElSelect> | null>(null)

watch(selectedType, (newType) => {
    if (newType != props.searchMethod) {
        emits('update-search-method', newType)
    }
})

watch(() => props.searchMethod, (newMethod) => {
    selectedType.value = newMethod
})


// 动态切换图标
const currentIcon = computed(() => {
    const iconMap = {
        prefix_search: BsSearch,                // 前缀搜索
        regex_search: BsFilterLeft,             // 正则搜索
        suggest_search: BsBinoculars,             // 前缀距离搜索
        prefix_distance_search: BsSearchHeart,   // 建议搜索
    }
    // Type assertion to fix TS error
    return iconMap[props.searchMethod as keyof typeof iconMap] || BsSearch
})

// 点击图标 → 打开下拉框
const openSelect = (e: MouseEvent | TouchEvent) => {
    (selectRef.value as InstanceType<typeof ElSelect> | null)?.toggleMenu()
    e.preventDefault()
}


</script>


<style scoped>
/* 隐藏原生下拉框，只保留弹出功能 */
:deep(.hidden-select) {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
    pointer-events: none;
}
</style>
