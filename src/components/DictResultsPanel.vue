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

  Two layouts (prop `anchorLayout`, macOS Dictionary inspired):

    bar   - horizontal anchor bar; dictionary links that do not fit the
            available width are hidden (visibility only, layout kept) and
            collected into an inline "more" dropdown that follows the bar
            (NOT floating). Link names truncate with an ellipsis past
            `--dict-anchor-link-max-width`.

    ball  - no anchor bar at all; only the locate button, docked to the
            bottom-right corner of this panel.

  Overflow detection is re-run whenever the bar is resized (ResizeObserver)
  or a new lookup arrives (lookupSeq watch).
-->
<template>
    <!-- Bar mode: horizontal anchor bar + inline overflow "more" button -->
    <div v-if="anchorLayout === 'bar'" ref="anchorBarRef" class="anchor-bar">
        <el-anchor :container="wordDetailScrollbarRef" :offset="10" direction="horizontal" class="anchor-dict">
            <el-anchor-link v-for="(_, dictName) in controller.lookupResults" :key="dictName"
                :href="`#dict-iframe-container-${dictName}`"
                :class="{ 'is-overflowed': overflowDictNames.includes(dictName) }"
                @click.prevent="scrollToDictionary(dictName)">
                <div class="anchor-link-content">
                    <el-image :src="getDictCover(dictName)" class="dropdown-custom-icon">
                        <template #error>
                            <BiSolidBookBookmark :size="25" />
                        </template>
                    </el-image>
                    <el-text class="anchor-link-name">
                        {{ dictName }}
                    </el-text>
                </div>
            </el-anchor-link>
        </el-anchor>

        <!-- Overflowed dictionaries live in this inline "more" menu. -->
        <el-dropdown v-show="overflowDictNames.length > 0" placement="bottom-end" popper-class="vibrant-dropdown"
            @command="scrollToDictionary">
            <el-button text circle bg class="locate-dict-button anchor-more-button">
                <el-icon>
                    <MoreFilled />
                </el-icon>
            </el-button>
            <template #dropdown>
                <el-dropdown-menu>
                    <el-dropdown-item v-for="dictName in overflowDictNames" :key="dictName" :command="dictName">
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
    </div>

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
            <el-collapse-item v-for="(htmlList, dictName) in controller.lookupResults"
                :key="`${controller.lookupSeq}-${dictName}`" :id="`dict-iframe-container-${dictName}`"
                class="dict-iframe-container" :title="dictName" :name="dictName" :is-active="true">
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
                        @location-click="handleLocationClick" @keydown="emit('iframe-keydown', $event)"
                        @context-menu="emit('context-menu', $event)" />
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

    <!-- Ball mode: only the locate button, docked bottom-right of this panel.
         (`.el-splitter` is the nearest positioned ancestor; the results panel
         is its rightmost full-height panel, so bottom-right lands inside it.) -->
    <el-dropdown v-if="anchorLayout === 'ball'" placement="bottom-end" popper-class="vibrant-dropdown"
        @command="scrollToDictionary">
        <el-button text circle bg class="locate-dict-button anchor-ball-button">
            <el-icon>
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
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
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
    /**
     * 'bar'  => horizontal anchor bar + inline overflow "more" button
     * 'ball' => only the locate button (docked bottom-right of the panel)
     */
    anchorLayout: {
        type: String as PropType<'bar' | 'ball'>,
        default: 'ball',
    },
})

const emit = defineEmits<{
    (e: 'entry-click', entryPath: string): void
    (e: 'iframe-keydown', evt: unknown): void
    (e: 'context-menu', payload: { selectedText: string; x: number; y: number }): void
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

// --- Anchor bar overflow ("more" dropdown) ---

/** Reserve this much bar width for the inline "more" button. */
const MORE_BUTTON_RESERVED_PX = 44

/** Dictionary names that do not fit the anchor bar (shown in the menu). */
const overflowDictNames = ref<string[]>([])

const anchorBarRef = ref<HTMLElement>()
let measureRaf = 0
let resizeObserver: ResizeObserver | null = null

/**
 * Recompute which dictionary links overflow the bar. Overflowed links keep
 * their layout (visibility:hidden, not display:none), so measuring is stable
 * and there is no reflow flash; the clipped tail is simply invisible, like
 * the macOS Dictionary anchor bar.
 */
const measureOverflow = (): void => {
    if (props.anchorLayout !== 'bar') {
        overflowDictNames.value = []
        return
    }
    cancelAnimationFrame(measureRaf)
    measureRaf = requestAnimationFrame(() => {
        const wrap = anchorBarRef.value
        const names = Object.keys(props.controller.lookupResults)
        if (!wrap || !names.length) {
            overflowDictNames.value = []
            return
        }
        const links = Array.from(wrap.querySelectorAll<HTMLElement>('.el-anchor__link'))
        const wrapRect = wrap.getBoundingClientRect()
        const avail = wrapRect.width - MORE_BUTTON_RESERVED_PX
        const overflow: string[] = []
        links.forEach((link, i) => {
            const name = names[i]
            if (!name) return
            if (link.getBoundingClientRect().right - wrapRect.left > avail) {
                overflow.push(name)
            }
        })
        overflowDictNames.value = overflow
    })
}

onMounted(() => {
    const wrap = anchorBarRef.value
    if (wrap && typeof ResizeObserver !== 'undefined') {
        resizeObserver = new ResizeObserver(() => measureOverflow())
        resizeObserver.observe(wrap)
    }
    measureOverflow()
})

onBeforeUnmount(() => {
    resizeObserver?.disconnect()
    resizeObserver = null
    cancelAnimationFrame(measureRaf)
})

// A fresh lookup re-renders the anchor links -> re-measure.
watch(() => props.controller.lookupSeq, () => measureOverflow())

// Switching layouts toggles the bar itself; re-measure once it is mounted.
watch(
    () => props.anchorLayout,
    (mode) => {
        if (mode === 'bar') {
            nextTick(() => measureOverflow())
        } else {
            overflowDictNames.value = []
            cancelAnimationFrame(measureRaf)
        }
    }
)

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
/* ============ Anchor bar (bar layout) ============ */
.anchor-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-bottom: 1px solid var(--el-border-color-light);
}

.anchor-bar :deep(.el-anchor) {
    flex: 1;
    min-width: 0;
}

.anchor-bar :deep(.el-anchor--horizontal .el-anchor__list) {
    display: flex;
    flex-wrap: nowrap;
    overflow: hidden;
}

/* Overflowed links keep layout space but are invisible (clipped tail). */
.anchor-bar :deep(.el-anchor__item.is-overflowed) {
    visibility: hidden;
}

/* One link: icon + name, capped width, name truncated with "…". */
.anchor-link-content {
    display: flex;
    align-items: center;
    gap: 4px;
    max-width: var(--dict-anchor-link-max-width, 180px);
}

.anchor-link-name {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* The "more" button follows the anchor bar inline (NOT floating). */
.anchor-more-button {
    flex-shrink: 0;
    margin-left: auto;
}

/* ============ Ball layout ============ */
/* Docked inside this panel (the splitter is the nearest positioned
   ancestor; the results panel is its rightmost full-height panel). */
.anchor-ball-button {
    position: absolute;
    right: 20px;
    bottom: 20px;
    z-index: 20;
}

/* ============ Results area (shared) ============ */
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
