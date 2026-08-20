<template>
    <div class="icon-select-wrapper">
        <el-dropdown trigger="click" @command="handleSelect">
            <span class="dropdown-trigger">
                <component :is="currentIcon" class="prefix-icon" />
            </span>

            <template #dropdown>
                <el-dropdown-menu>
                    <el-dropdown-item v-for="option in searchOptions" :key="option.value" :command="option.value">
                        <el-icon size="20">
                            <component :is="option.icon" />
                        </el-icon>
                        <span>{{ option.label }}</span>
                    </el-dropdown-item>
                </el-dropdown-menu>
            </template>
        </el-dropdown>
    </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { BsSearch } from 'vue-icons-plus/bs'
import { VscRegex, VscSearchFuzzy } from 'vue-icons-plus/vsc'
import { Fa6Searchengin } from 'vue-icons-plus/fa6'
import { useI18n } from 'vue-i18n'

interface SearchOption {
    value: string
    label: string
    icon: unknown
}

const props = defineProps<{
    searchMethod: string
}>()

const emit = defineEmits<{
    (e: 'update-search-method', method: string): void
}>()

const { t } = useI18n()

const searchOptions: SearchOption[] = [
    { value: 'prefix_search', label: t('searchMethod.prefix'), icon: BsSearch },
    { value: 'regex_search', label: t('searchMethod.regex'), icon: VscRegex },
    { value: 'prefix_distance_search', label: t('searchMethod.prefixDistance'), icon: Fa6Searchengin },
    { value: 'suggest_search', label: t('searchMethod.fuzzy'), icon: VscSearchFuzzy },
]

const currentIcon = computed(() => {
    const found = searchOptions.find((o) => o.value === props.searchMethod)
    return found?.icon || BsSearch
})

const handleSelect = (command: string): void => {
    if (command !== props.searchMethod) {
        emit('update-search-method', command)
    }
}
</script>

<style scoped>
.dropdown-trigger {
    cursor: pointer;
    display: inline-flex;
    align-items: center;
}

.prefix-icon {
    width: 16px;
}
</style>