<template>
    <iframe ref="iframeRef" class="dict-iframe" frameborder="0" scrolling="no"></iframe>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onUnmounted, computed } from 'vue'
import debounce from 'lodash/debounce'
import {
    API_BASE_URL,
    IFRAME_HEIGHT_PADDING,
    IFRAME_HEIGHT_DEBOUNCE_MS,
    URL_SCHEME,
    IFRAME_MSG,
} from '@/common/constants'

interface Props {
    dictionaryName: string
    index: number
    html: string
    cssUrls: string[]
    jsUrls: string[]
    basePath: string
    dictionaryRoot: string
    isDark: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
    (e: 'entry-click', path: string): void
    (e: 'keydown', event: unknown): void
}>()

const iframeRef = ref<HTMLIFrameElement | null>(null)
const iframeId = computed(() => `${props.dictionaryName}-${props.index}`)
const tailElementId = computed(() => `${props.dictionaryName}-dict-tail`)

const baseUrl = computed(() => `${API_BASE_URL}/${encodeURIComponent(props.dictionaryName)}`)

let mutationObserver: MutationObserver | null = null
let isUpdatingHeight = false
let messageListener: ((e: MessageEvent) => void) | null = null

// ============== Dark Mode Injection ==============
function updateDarkMode(isDark: boolean): void {
    const doc = iframeRef.value?.contentDocument
    if (!doc) return

    let styleEl = doc.getElementById('dict-custom-style') as HTMLStyleElement | null
    if (!styleEl) {
        styleEl = doc.createElement('style')
        styleEl.id = 'dict-custom-style'
        doc.head.appendChild(styleEl)
    }

    styleEl.textContent = isDark
        ? `html { filter: invert(0.92) hue-rotate(180deg); }
       img { filter: invert(0.92) hue-rotate(180deg) contrast(1.05); }`
        : ''
}

// ============== Content Rendering ==============
function processHtml(rawHtml: string): string {
    return rawHtml
        .replace(/<\/?(html|head|body)[^>]*>/gi, '')
        .replace(/<link[^>]*>/gi, '')
        .replace(/<script[^>]*>[\s\S]*?<\/script>/gi, '')
        .replace(/file:\//g, '')
        .replace(/src="/g, `src="${baseUrl.value}/`)
        .replace(/href="(?!https?:\/\/|sound:|entry:|#|help:|d:|x:|adddr:|addexample:|helpp:)/g, `href="${baseUrl.value}/`)
}

function injectStyles(doc: Document): void {
    const darkStyle = doc.createElement('style')
    darkStyle.id = 'dict-custom-style'
    darkStyle.textContent = props.isDark
        ? `html { filter: invert(0.92) hue-rotate(180deg); }
       img { filter: invert(0.92) hue-rotate(180deg) contrast(1.05); }`
        : ''
    doc.head.appendChild(darkStyle)

    if (props.cssUrls?.length) {
        for (const cssUrl of props.cssUrls) {
            const link = doc.createElement('link')
            link.rel = 'stylesheet'
            link.href = `${API_BASE_URL}/${encodeURI(cssUrl)}`
            doc.head.appendChild(link)
        }
    }
}

function injectScripts(doc: Document): Promise<void> {
    if (!props.jsUrls?.length) return Promise.resolve()

    return props.jsUrls.reduce((prevPromise, jsUrl) => {
        return prevPromise.then(() => {
            return new Promise<void>((resolve) => {
                const script = doc.createElement('script')
                script.src = `${API_BASE_URL}/${encodeURI(jsUrl)}`
                script.charset = 'UTF-8'
                script.onload = () => resolve()
                script.onerror = () => {
                    console.warn(`Script load failed: ${jsUrl}`)
                    resolve()
                }
                doc.head.appendChild(script)
            })
        })
    }, Promise.resolve())
}

function injectClickHandler(doc: Document): void {
    const script = doc.createElement('script')
    script.textContent = `
    document.addEventListener('click', (e) => {
      const a = e.target.closest('a[href]');
      if (!a) return;
      const href = a.href || a.getAttribute('href');

      if (href.startsWith('${URL_SCHEME.ENTRY}')) {
        e.preventDefault();
        const raw = a.getAttribute('href') || a.href;
        window.parent.postMessage({
          type: '${IFRAME_MSG.ENTRY_CLICK}',
          iframeId: '${iframeId.value}',
          entry: raw.replace('${URL_SCHEME.ENTRY}', '')
        }, '*');
      }
      else if (href.startsWith('${URL_SCHEME.SOUND}')) {
        e.preventDefault();
        window.parent.postMessage({
          type: '${IFRAME_MSG.SOUND_CLICK}',
          iframeId: '${iframeId.value}',
          sound: encodeURIComponent(href.replace('${URL_SCHEME.SOUND}', ''))
        }, '*');
      }
      else if (href.includes('#') && href.includes('127.0.0.1')) {
        e.preventDefault();
        const hash = href.split('#')[1];
        const el = document.getElementById(hash);
        if (el) {
          window.parent.postMessage({
            type: '${IFRAME_MSG.LOCATION_CLICK}',
            iframeId: '${iframeId.value}',
            elementOffsetTop: el.offsetTop
          }, '*');
        }
      }
      else {
        e.preventDefault();
      }
    });
  `
    doc.body.appendChild(script)
}

function injectKeydownHandler(doc: Document): void {
    const script = doc.createElement('script')
    script.textContent = `
    document.addEventListener('keydown', (e) => {
      window.parent.postMessage({
        type: 'KEYDOWN',
        key: e.key,
        code: e.code,
        ctrlKey: e.ctrlKey,
        shiftKey: e.shiftKey,
        altKey: e.altKey,
        metaKey: e.metaKey,
        iframeId: '${iframeId.value}'
      }, '*');
    });
  `
    doc.body.appendChild(script)
}

async function renderIframe(): Promise<void> {
    const iframe = iframeRef.value
    if (!iframe) return

    const doc = iframe.contentDocument || iframe.contentWindow?.document
    if (!doc) return

    const isFirstRender = !doc.getElementById('dict-custom-style')

    if (isFirstRender) {
        doc.body.innerHTML = ''
        injectStyles(doc)
        await injectScripts(doc)
        injectClickHandler(doc)
        injectKeydownHandler(doc)
    }

    doc.body.innerHTML = processHtml(props.html)

    const tail = doc.createElement('p')
    tail.id = tailElementId.value
    tail.textContent = ''
    tail.style.cssText = 'margin:0;padding:0;height:0;visibility:hidden;'
    doc.body.appendChild(tail)

    await nextTick()
    updateIframeHeightDebounced()
}

// ============== Height Management ==============
function updateIframeHeight(): void {
    const iframe = iframeRef.value
    const doc = iframe?.contentDocument
    if (!iframe || !doc) return

    const tailEl = doc.getElementById(tailElementId.value)
    if (!tailEl) return

    const bottom = tailEl.getBoundingClientRect().bottom
    iframe.style.height = `${bottom + IFRAME_HEIGHT_PADDING}px`
}

const updateIframeHeightDebounced = debounce(updateIframeHeight, IFRAME_HEIGHT_DEBOUNCE_MS)

function setupMutationObserver(iframe: HTMLIFrameElement): void {
    const doc = iframe.contentDocument
    if (!doc) return

    mutationObserver = new MutationObserver(() => {
        if (isUpdatingHeight) return
        isUpdatingHeight = true

        updateIframeHeight()

        setTimeout(() => {
            isUpdatingHeight = false
        }, 100)
    })

    mutationObserver.observe(doc.body, {
        childList: true,
        subtree: true,
        attributes: true,
        characterData: true,
    })
}

// ============== Message Handling ==============
function handleEntryClick(entry: string): void {
    try {
        emit('entry-click', decodeURIComponent(entry))
    } catch {
        emit('entry-click', entry)
    }
}

function handleSoundClick(sound: string): void {
    const soundUrl = `${baseUrl.value}/${sound}`
    const audio = new Audio(soundUrl)
    audio.currentTime = 0
    audio.play().catch(err => console.warn('Audio playback failed:', err))
}

function handleLocationClick(offsetTop: number): void {
    const scrollContainer = document.querySelector('.word-detail') as HTMLElement | null
    const iframeEl = document.getElementById(`dict-iframe-container-${props.dictionaryName}`)
    if (!scrollContainer || !iframeEl) return

    const iframeTop = iframeEl.getBoundingClientRect().top - scrollContainer.getBoundingClientRect().top
    const targetScrollTop = scrollContainer.scrollTop + iframeTop + offsetTop

    scrollContainer.scrollTo({
        top: targetScrollTop,
        behavior: 'instant',
    })
}

function setupMessageListener(): void {
    messageListener = (e: MessageEvent) => {
        if (e.data?.iframeId !== iframeId.value) return

        switch (e.data.type) {
            case IFRAME_MSG.ENTRY_CLICK:
                handleEntryClick(e.data.entry)
                break
            case IFRAME_MSG.SOUND_CLICK:
                handleSoundClick(e.data.sound)
                break
            case IFRAME_MSG.LOCATION_CLICK:
                handleLocationClick(e.data.elementOffsetTop)
                break
            case IFRAME_MSG.KEYDOWN:
                emit('keydown', e.data)
                break
        }
    }

    window.addEventListener('message', messageListener)
}

// ============== Watchers ==============
watch(() => props.isDark, updateDarkMode)

watch(
    () => [props.html, props.basePath],
    async () => {
        await nextTick()
        await renderIframe()
    },
    { deep: true, immediate: true }
)

watch(iframeRef, (val) => {
    if (mutationObserver) {
        mutationObserver.disconnect()
        mutationObserver = null
    }
    if (val) {
        nextTick(() => setupMutationObserver(val))
    }
}, { immediate: true })

// ============== Lifecycle ==============
onMounted(() => {
    window.addEventListener('resize', updateIframeHeightDebounced)
    setupMessageListener()
})

onUnmounted(() => {
    window.removeEventListener('resize', updateIframeHeightDebounced)
    mutationObserver?.disconnect()

    if (messageListener) {
        window.removeEventListener('message', messageListener)
    }

    if (iframeRef.value) {
        iframeRef.value.srcdoc = ''
    }
})
</script>

<style scoped>
.dict-iframe {
    width: 100%;
    border: none;
}
</style>
