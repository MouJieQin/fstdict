import { ref, nextTick } from 'vue'
import type { Ref } from 'vue'

interface UseSplitterResizeOptions {
    splitterRef: Ref<any>
    expandWidth: number | string
}

export function useSplitterResize(options: UseSplitterResizeOptions) {
    const { splitterRef, expandWidth } = options

    // --- Splitter sizing (local UI state of this tab) ---
    const panelSize = ref<number | string>(expandWidth)
    // const splitterRef = ref<any>(null)
    /** Minimum width of the word-options panel (kept 100px while dragging). */
    const MIN_PANEL_SIZE = 180
    const panelMin = ref(MIN_PANEL_SIZE)
    const RESIZE_COLLAPSE_THRESHOLD_PX = MIN_PANEL_SIZE + 1
    /** Duration of a programmatic resize animation, in ms (matches the CSS). */
    const ANIMATION_MS = 240
    /**
     * If the user drags the word-options panel narrower than this many pixels,
     * the drag is finished and the panel folds away automatically. Tune freely.
     */
    /** The rendered `.el-splitter-panel` element of the word-options panel. */
    const wordOptionsPanelEl = (): HTMLElement | null =>
        splitterRef.value?.$el?.querySelector('.el-splitter-panel') ?? null
    /**
     * Temporarily enable a CSS transition on the panel so programmatic resizes
     * (expand / collapse) ease smoothly. The class is removed shortly after, so
     * manual bar dragging stays 1:1 with the pointer (no rubber-band lag).
     */
    const withPanelAnimation = (): void => {
        const panelEl = wordOptionsPanelEl()
        if (!panelEl) return
        panelEl.classList.add('panel-animating')
        window.setTimeout(() => panelEl.classList.remove('panel-animating'), ANIMATION_MS + 80)
    }
    const handlePanelResize = (size: number | string): void => {
        panelSize.value = size
    }
    const expandPanel = async (): Promise<void> => {
        if (Number(panelSize.value) <= 5) {
            withPanelAnimation()
            panelSize.value = expandWidth
            await nextTick()
        }
    }
    const triggerPanel = (): void => {
        if (Number(panelSize.value) <= 5) {
            expandPanel()
        } else {
            collapsePanel()
        }
    }
    /**
     * Smoothly fold the word-options panel to exactly 0px.
     * `min="100"` would clamp a `size: 0` prop back to 100 (the panel's prop
     * watcher clamps on every external size change), so we temporarily drop the
     * min for one tick and restore it after the animation finishes.
     */
    const collapsePanel = (): void => {
        if (panelMin.value !== 0) {
            panelMin.value = 0
            window.setTimeout(() => {
                panelMin.value = MIN_PANEL_SIZE
            }, ANIMATION_MS + 120)
        }
        withPanelAnimation()
        panelSize.value = 0
    }


    const onSplitterCollapse = (index: number, type: 'start' | 'end', sizes: number[]): void => {
        withPanelAnimation()
        // console.log("onSplitterCollapse", index, type, sizes)
    }

    /**
     * While the splitter bar is being dragged, the webview happily selects a
     * large page region under the pointer. Block text selection document-wide
     * for the whole drag (restored on `resize-end`).
     */
    const onSplitterResizeStart = (): void => {
        document.body.classList.add('panel-resizing')
    }
    /**
     * Restore selection after the drag. As a safety net, if the panel ended up
     * narrower than the threshold (user dragged it (almost) shut), stop the
     * drag by folding the panel away entirely.
     */
    const onSplitterResizeEnd = (): void => {
        document.body.classList.remove('panel-resizing')
        const panelEl = wordOptionsPanelEl()
        if (!panelEl) return
        const widthPx = Number(panelSize.value)
        if (widthPx > 0 && widthPx < RESIZE_COLLAPSE_THRESHOLD_PX) {
            collapsePanel()
        }
    }

    return {
        panelMin,
        panelSize,
        handlePanelResize,
        expandPanel,
        collapsePanel,
        triggerPanel,
        onSplitterCollapse,
        onSplitterResizeStart,
        onSplitterResizeEnd,
    }
}