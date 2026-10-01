<!--
  DictTabs.vue
  ------------------------------------------------------------------
  Tab bar of the dictionary page.

  Responsibilities (UI only):
  - Render the tab strip (Element Plus "card" tabs).
  - Drag & drop reordering backed by SortableJS.
  - Emit add-tab / close-tab so the PARENT decides how a new session is
    created (the store only keeps tab metadata; requesting a backend
    session is an orchestration concern of DictPage).

  The panes are intentionally empty: each tab's real content is rendered
  by its own DictTabSession (own WebSocket + WordOptions + results panel),
  which stays mounted and is toggled with v-show.
-->
<template>
    <el-tabs v-model="activeId" type="card" editable class="dict-tabs" ref="tabRef" @edit="handleTabsEdit">
        <el-tab-pane v-for="tab in store.tabs" :key="tab.id" :name="tab.id" :label="ellipsisLabel(tab.title)" />
    </el-tabs>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import Sortable from 'sortablejs'
import { useGetDerivedNamespace } from 'element-plus'
import type { TabsInstance, TabPaneName } from 'element-plus'
import { useDictTabsStore } from '@/stores/dictTabs'

const store = useDictTabsStore()

// The parent owns tab lifecycle: "add" must ask the backend for a new
// session before a tab can be created, so we bubble the raw intent up.
const emit = defineEmits<{
    (e: 'add-tab'): void
    (e: 'close-tab', tabId: string): void
}>()

// Two-way binding with the store's active tab id.
const activeId = computed<string>({
    get: () => store.activeTabId,
    set: (id: string) => store.activateTab(id),
})

const ellipsisLabel = (label: string): string => {
    if (label.length <= 20) return label
    return `${label.slice(0, 20)}...`
}


const tabRef = ref<TabsInstance>()
const ns = useGetDerivedNamespace().value

let sortableInstance: Sortable | null = null

/**
 * (Re-)bind SortableJS to the tab list DOM produced by Element Plus.
 * Must be re-run after the DOM changes (mount, add/remove tabs).
 */
const initSortable = (): void => {
    if (sortableInstance) {
        sortableInstance.destroy()
        sortableInstance = null
    }

    // Element Plus stores the tab <ul> and the animated bar on the tabs nav ref.
    const tabListRef = tabRef.value?.tabNavRef?.tabListRef
    if (!tabListRef) return

    sortableInstance = new Sortable(tabListRef, {
        animation: 150,
        draggable: `.${ns}-tabs__item`,
        // Do not start a drag from a disabled tab or from the close button.
        filter: '.is-disabled, .el-tabs__item .el-icon-close',
        // Custom fallback drag (not native HTML5 DnD) -> consistent ghosting
        // inside a Tauri webview and no interference with window drag regions.
        forceFallback: true,

        onEnd: (event) => {
            const { oldIndex, newIndex } = event
            if (oldIndex === undefined || newIndex === undefined || oldIndex === newIndex) return

            // 1. Persist the new order in the store (single source of truth).
            store.reorderTabs(oldIndex, newIndex)

            // 2. Force Element Plus to re-render the tab bar line in its new spot
            //    (otherwise the blue underline sticks to the old position).
            nextTick(() => {
                tabRef.value?.tabNavRef?.tabBarRef?.update()
            })
        },
    })
}

/** Add / close intents from the editable tab bar (the "+" button and close icons). */
const handleTabsEdit = (targetName: TabPaneName | undefined, action: 'remove' | 'add'): void => {
    if (action === 'add') {
        emit('add-tab')
    } else if (action === 'remove' && targetName != null) {
        emit('close-tab', String(targetName))
    }
}

// el-tabs re-renders its DOM whenever the tab list changes, so re-bind
// Sortable after a tab is added or removed (pure re-ordering is handled
// inside onEnd and does not change the list length).
watch(
    () => store.tabs.length,
    async () => {
        await nextTick()
        initSortable()
    }
)

onMounted(() => {
    initSortable()
})

onBeforeUnmount(() => {
    sortableInstance?.destroy()
    sortableInstance = null
})
</script>

<style scoped>
/* The panes are pure labels; the content is rendered by DictResultsPanel,
   so hide el-tabs' content wrapper entirely. */

:deep(.dict-tabs .el-tabs__content) {
    display: none;
}

:deep(.dict-tabs .el-tabs__header) {
    margin-bottom: 0;
}

:deep(.el-tabs__header) {
    height: var(--tab-height);
    margin: 0;
    padding-right: 1rem;
    border: none;
    overflow: hidden;
}

:deep(.el-tabs__item) {
    height: var(--tab-height);
    line-height: var(--tab-height);
    background-color: var(--tab-bg);
    color: var(--el-text-color-regular);
}

:deep(.el-tabs__item:hover) {
    background-color: var(--tab-hover-bg);
    color: var(--el-text-color-regular);

}

:deep(.el-tabs__item.is-active) {
    background-color: var(--tab-active-bg);
    color: var(--el-text-color-regular);
    border: none;
}

:deep(.el-tabs__nav-prev),
:deep(.el-tabs__nav-next) {
    height: var(--tab-height);
    top: 0;
    display: flex;
    align-items: center;
    justify-content: center;
}

:deep(.el-tabs__nav-wrap) {
    height: 100%;
}
</style>
