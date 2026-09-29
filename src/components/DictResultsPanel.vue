<!--
  DictResultsPanel.vue
  ------------------------------------------------------------------
  Renders the result area for ONE dictionary tab.

  One instance is mounted per tab (DictTabSession keeps them alive with
  v-show), so all UI state that only this panel needs - the collapse
  selection and the scroll offset - lives here as local refs and simply
  persists while the tab is hidden.

  Data comes from the tab's `controller` prop (see TabController), which
  is the same reactive object the shared TitleBar reads, so the panel
  always shows "this tab's" lookup results.
-->
<template>
    <el-scrollbar class="word-detail" :class="{ 'anki-mode': env === 'anki', 'not-anki-mode': env !== 'anki' }"
        ref="wordDetailScrollbarRef" always>
        <el-collapse class="sticky-collapse" expand-icon-position="left" v-model="activeNames">
            <!-- Note panel (markdown, rendered client-side) -->
            <el-collapse-item v-if="controller.noteContent" :title="$t('dictPage.myNotes')" name="notes"
                :is-active="true" class="dict-iframe-container">
                <template #icon="{ isActive }">
                    <el-icon v-show="!isActive" class="el-collapse-item__arrow">
                        <CaretRight />
                    </el-icon>
                    <el-icon v-show="isActive" class="el-collapse-item__arrow">
                        <CaretBottom />
                    </el-icon>
                    <BiSolidBookBookmark size="35" />
                </template>
                <div class="markdown-note-content" v-html="md.render(controller.noteContent ?? '')"></div>
            </el-collapse-item>

            <!-- One collapse section per dictionary -->
            <el-collapse-item v-for="(htmlList, dictName) in controller.lookupResults" :key="dictName"
                :id="`dict-iframe-container-${dictName}`" class="dict-iframe-container" :title="dictName"
                :name="dictName" :is-active="true">
                <template #icon="{ isActive }">
                    <el-icon v-show="!isActive" class="el-collapse-item__arrow">
                        <CaretRight />
                    </el-icon>
                    <el-icon v-show="isActive" class="el-collapse-item__arrow">
                        <CaretBottom />
                    </el-icon>
                    <el-image :src="getDictCover(dictName)" class="collapse-custom-icon">
                        <template #error>
                            <BiSolidBookBookmark size="35" />
                        </template>
                    </el-image>
                </template>

                <div v-for="(html, index) in htmlList" :key="index">
                    <div class="simple-divider"></div>
                    <DictIframe :dictionary-name="dictName" :index="index" :html="html"
                        :css-urls="controller.dictsInfo[dictName]?.css || []"
                        :js-urls="controller.dictsInfo[dictName]?.js || []"
                        :base-path="controller.dictsInfo[dictName]?.data || ''"
                        :dictionary-root="controller.dictsInfo[dictName]?.root || ''"
                        :is-dark="systemConfigStore.isDark" @entry-click="emit('entry-click', $event)"
                        @location-click="handleLocationClick" @keydown="emit('iframe-keydown', $event)" />
                </div>
            </el-collapse-item>
        </el-collapse>

        <!-- Empty state: nothing searched yet in this tab -->
        <div v-show="!controller.lastSearchKeyword && !controller.hasResultLastSearch" class="empty-state">
            <p class="dict-homepage-type-p">{{ $t('dictPage.typeToLookup') }}</p>
            <br />
            <p v-if="showAddDictInfo" class="dict-homepage-type-p">
                {{ $t('dictPage.noActiveDicts') }}
            </p>
            <p v-for="dict in activeDictionaries" :key="dict.name" class="dict-homepage-dict-p">
                {{ dict.name }}
            </p>
        </div>

        <!-- Empty state: searched but the backend returned no results -->
        <div v-show="controller.lastSearchKeyword && !controller.hasResultLastSearch" class="empty-state">
            <p class="dict-homepage-type-p">
                {{ $t('dictPage.noResults', { word: controller.lastSearchKeyword }) }}
            </p>
            <br />
            <p v-if="showAddDictInfo" class="dict-homepage-type-p">
                {{ $t('dictPage.noActiveDicts') }}
            </p>
            <p v-for="dict in activeDictionaries" :key="dict.name" class="dict-homepage-dict-p">
                {{ dict.name }}
            </p>
        </div>
    </el-scrollbar>

    <!-- Floating "locate dictionary" dropdown -->
    <el-dropdown placement="bottom-end" @command="scrollToDictionary" popper-class="vibrant-dropdown">
        <el-button text class="locate-dict-button" circle bg style="background: var(--glass-bg);">
            <el-icon class="el-icon--right">
                <MoreFilled />
            </el-icon>
        </el-button>
        <template #dropdown>
            <el-dropdown-menu>
                <el-dropdown-item v-for="(_, dictName) in controller.lookupResults" :key="dictName" :command="dictName">
                    <el-image :src="getDictCover(dictName)" class="dropdown-custom-icon">
                        <template #error>
                            <BiSolidBookBookmark :size="25" />
                        </template>
                    </el-image>
                    {{ dictName }}
                </el-dropdown-item>
            </el-dropdown-menu>
        </template>
    </el-dropdown>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import type { PropType } from 'vue'
import MarkdownIt from 'markdown-it'
import type { ScrollbarInstance } from 'element-plus'
import type { TabController } from '@/stores/dictTabs'
import { useSystemConfigStore } from '@/stores'
import DictIframe from '@/components/DictIframe.vue'
import { BiSolidBookBookmark } from 'vue-icons-plus/bi'
import { CaretRight, CaretBottom, MoreFilled } from '@element-plus/icons-vue'

const props = defineProps({
    /** Runtime state of the tab this panel belongs to. */
    controller: {
        type: Object as PropType<TabController>,
        required: true,
    },
    env: {
        type: String,
        default: '',
    },
})

const emit = defineEmits<{
    (e: 'entry-click', entryPath: string): void
    (e: 'iframe-keydown', evt: unknown): void
}>()

const systemConfigStore = useSystemConfigStore()

// Markdown renderer (local: only this panel renders notes).
const md = new MarkdownIt({
    breaks: true,
    xhtmlOut: true,
})

// Collapse selection: local per-panel UI state (persists via v-show).
const activeNames = ref<string[]>([])

const wordDetailScrollbarRef = ref<ScrollbarInstance>()

const activeDictionaries = computed(() =>
    props.controller.sessionDictsSettingInfo.filter((d) => d.is_enabled)
)

const showAddDictInfo = computed(() => !activeDictionaries.value.length)

const getDictCover = (dictName: string): string =>
    props.controller.dictsInfo[dictName]?.cover_url || ''

/**
 * A new lookup in this tab (controller.lookupSeq was bumped):
 * rebuild the collapse selection from the fresh results and scroll to top.
 */
watch(
    () => props.controller.lookupSeq,
    () => {
        activeNames.value = Object.keys(props.controller.lookupResults)
        if (props.controller.noteContent) activeNames.value.unshift('notes')

        nextTick(() => {
            if (props.env === 'anki') {
                window.scrollTo(0, 0)
            } else {
                wordDetailScrollbarRef.value?.scrollTo(0, 0)
            }
        })
    }
)

/** Expand a dictionary section and scroll it into view (locate dropdown). */
const scrollToDictionary = async (dictName: string): Promise<void> => {
    const element = document.getElementById(`dict-iframe-container-${dictName}`)
    if (!element) return

    if (!activeNames.value.includes(dictName)) {
        activeNames.value = [...activeNames.value, dictName]
    }

    await nextTick()

    const scrollbar = wordDetailScrollbarRef.value
    if (!scrollbar) return
    const scrollWrap = scrollbar.wrapRef
    if (!scrollWrap) return

    const wrapRect = scrollWrap.getBoundingClientRect()
    const targetRect = element.getBoundingClientRect()
    const targetScrollTop = scrollWrap.scrollTop + (targetRect.top - wrapRect.top)

    scrollWrap.scrollTo({
        top: targetScrollTop,
        behavior: 'instant',
    })
}

/** Jump to an anchor offset inside a dictionary iframe. */
const handleLocationClick = (dictionaryName: string, offsetTop: number): void => {
    const scrollbar = wordDetailScrollbarRef.value
    if (!scrollbar || !scrollbar.wrapRef) return
    const wrap = scrollbar.wrapRef

    const iframeEl = document.getElementById(`dict-iframe-container-${dictionaryName}`)
    if (!iframeEl) return

    const wrapRect = wrap.getBoundingClientRect()
    const iframeRect = iframeEl.getBoundingClientRect()
    const iframeTopRelative = iframeRect.top - wrapRect.top

    const targetScrollTop = wrap.scrollTop + iframeTopRelative + offsetTop
    wrap.scrollTo({
        top: targetScrollTop,
        behavior: 'instant',
    })
}
</script>

<style scoped>
:deep(.collapse-custom-icon) {
    flex-shrink: 0;
    width: 2rem;
    height: 2rem;
    margin-right: 8px;
    vertical-align: middle;
}

:deep(.el-collapse-item__arrow) {
    flex-shrink: 0;
}

:deep(.sticky-collapse) {
    border: none;
}

:deep(.sticky-collapse .el-collapse-item__header) {
    background: transparent;
    white-space: nowrap;
    overflow: hidden;
}

.dict-iframe-container :deep(.el-collapse-item__content) {
    background-color: transparent;
}

.dict-iframe-container :deep(.el-collapse-item__wrap) {
    background-color: transparent;
}

.empty-state {
    text-align: center;
}
</style>
