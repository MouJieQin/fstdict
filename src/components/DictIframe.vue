<template>
  <!-- {{ doc_content }} -->
  <iframe ref="iframeRef" class="dict-iframe" frameborder="0" scrolling="no"
    sandbox="allow-scripts allow-same-origin"></iframe>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onUnmounted } from 'vue'

interface Props {
  dictionaryName: string
  html: string
  cssUrls: string[]
  jsUrls: string[]
  basePath: string
  dictionaryRoot: string
}

const props = defineProps<Props>()
const emits = defineEmits(['entry-click', 'keydown'])

const iframeRef = ref<HTMLIFrameElement | null>(null)
const API_PREFIX = 'http://localhost:5959/api/download?path='
// const baseUrl = ref(`${API_PREFIX}${props.basePath}`)
const baseUrl = ref(`${API_PREFIX}${encodeURIComponent(props.dictionaryName)}/data`)
const iframeId = ref(props.dictionaryRoot)
const doc_content = ref('')

// ================ 业务逻辑 ================
function handleEntryClick(entryPath: string) {
  console.log('✅ 点击词条:', entryPath)
  emits('entry-click', entryPath)
}

// ================ 核心渲染（极速版） ================
async function renderIframe() {
  const iframe = iframeRef.value
  if (!iframe) return

  const doc = iframe.contentDocument || iframe.contentWindow?.document
  if (!doc) return

  doc_content.value = props.html
    .replace(/file:\//g, '')
    .replace(/src=\"/g, `src="${baseUrl.value}/`)

  // <meta charset="UTF-8" />
  // const metaCharset = doc.querySelector('meta[charset]')
  // if (!metaCharset) {
  //   const meta = doc.createElement('meta')
  //   meta.charset = 'UTF-8'
  //   doc.head.appendChild(meta)
  // }

  const style = doc.createElement('style')
  style.textContent = `
    // html {
        // background: #1a1a2e !important;
        // color: #e5e7eb !important;
      // filter: invert(1) hue-rotate(180deg);
    // }
    // a { color: #818cf8 !important; }
    // h1, h2, h3, h4, h5, h6 { color: #f3f4f6 !important; }
    // table { border-color: #374151 !important; }
    // th, td { border-color: #374151 !important; }
    // /* 图片完全不动，保持原色 */
    // img { filter: none !important; }
      // body {
      //     filter: invert(1) hue-rotate(180deg);
      // }

      // img {
      //     filter: invert(1) hue-rotate(180deg) contrast(1.05);
      // }
      // body {
      //  padding-left: 1rem !important;
      //  padding-right: 1rem !important;
      // }
      // @media (max-width: 500px) {
      //   body {
      //     padding-left: 0 !important;
      //     padding-right: 0 !important;
      //   }
      // }
    `
  doc.head.appendChild(style)


  // 只在第一次加载 CSS/JS
  doc.body.innerHTML = ''

  // CSS
  if (props.cssUrls) {
    for (const cssUrl of props.cssUrls) {
      const link = doc.createElement('link')
      link.rel = 'stylesheet'
      link.href = `${API_PREFIX}${encodeURIComponent(cssUrl)}`
      doc.head.appendChild(link)
    }
  }

  // JS
  if (props.jsUrls) {
    for (const jsUrl of props.jsUrls) {
      const script = doc.createElement('script')
      script.src = `${API_PREFIX}${encodeURIComponent(jsUrl)}`
      script.charset = 'UTF-8'
      doc.head.appendChild(script)
    }
  }

  // 注入一次全局点击监听
  injectClickHandler(doc)
  // 注入一次全局键盘监听
  injectKeydownHandler(doc)

  await nextTick()

  // 只更新内容，不重建整个 iframe
  doc.body.innerHTML = doc_content.value
  const p = doc.createElement('p')
  p.textContent = "tail"
  p.id = props.dictionaryRoot + '-dict-tail'
  doc.body.appendChild(p)
  setTimeout(() => {
    updateIframeHeight()
  }, 100)
}

function injectKeydownHandler(doc: Document) {
  const script = doc.createElement('script')
  script.textContent = `
    document.addEventListener('keydown', (e) => {
      window.parent.postMessage({
        type: 'KEYDOWN',
        key: e.key,           // 按键名，比如 "Enter" "Escape"
        code: e.code,         // 按键码
        ctrlKey: e.ctrlKey,   // 组合键
        shiftKey: e.shiftKey,
        altKey: e.altKey,
        metaKey: e.metaKey,
        iframeId: '${iframeId.value}'
      }, '*');
    });
  `
  doc.body.appendChild(script)
}

// ================ 点击事件只注入一次 ================
function injectClickHandler(doc: Document) {
  const script = doc.createElement('script')
  script.textContent = `
    document.addEventListener('click', (e) => {
      const a = e.target.closest('a[href]');
      if (!a) return;
      const href = a.href || a.getAttribute('href');

      if (href.startsWith('entry://')) {
        e.preventDefault();
        window.parent.postMessage({
          type: 'ENTRY_CLICK',
          iframeId: '${iframeId.value}',
          entry: href.replace('entry://', '')
        }, '*');
      }
      else if (href.startsWith('sound://')) {
        e.preventDefault();
        window.parent.postMessage({
          type: 'SOUND_CLICK',
          iframeId: '${iframeId.value}',
          sound: encodeURIComponent(href.replace('sound://', ''))
        }, '*');
      }
      else if (href.includes('#') && href.includes('localhost')) {
          e.preventDefault();
          const hash = href.split('#')[1];
          const el = document.getElementById(hash);
          if (!el) return;
          window.parent.postMessage({
              type: 'LOCATION_CLICK',
              iframeId: '${iframeId.value}',
              elementOffsetTop: el.offsetTop
          }, '*');
      }
      else {
        e.preventDefault();
        console.log('拦截链接:', href);
      }
    });
  `
  doc.body.appendChild(script)
}

//  ================ 监听窗口resize ================
onMounted(() => {
  window.addEventListener('resize', updateIframeHeight)
})

onUnmounted(() => {
  window.removeEventListener('resize', updateIframeHeight)
})



// ================ 高度自适应 ================
function updateIframeHeight() {
  const iframe = iframeRef.value
  if (!iframe?.contentDocument) return
  const doc = iframe.contentDocument
  const realHeight = doc.getElementById(`${props.dictionaryRoot}-dict-tail`)?.getBoundingClientRect().bottom || 0
  // 赋值给 iframe
  iframe.style.height = `${realHeight + 10}px`

}

// ================ 监听变化 ================
watch(
  () => [props.html, props.basePath],
  async () => {
    await nextTick()
    renderIframe()
  },
  { deep: true, immediate: true }
)

// 🔥 自动高度（终极版，任何内容变化都能触发）
let mutationObserver: MutationObserver | null = null
const changedByThisCode = ref(false)

watch(iframeRef, (val) => {
  // 清理旧监听
  if (mutationObserver) {
    mutationObserver.disconnect()
    mutationObserver = null
  }
  if (!val) return

  nextTick(() => {
    const doc = val.contentDocument
    if (!doc) return

    // ==========================================
    // 🔥 监听 iframe 内部内容变化（变大变小都能触发）
    // ==========================================
    mutationObserver = new MutationObserver(() => {
      console.log(props.dictionaryRoot, "**resizeObserver is triggered:", doc.documentElement.scrollHeight)
      if (changedByThisCode.value) return
      changedByThisCode.value = true

      // 无论内容多少，直接取最新高度
      const realHeight = doc.getElementById(`${props.dictionaryRoot}-dict-tail`)?.getBoundingClientRect().bottom || 0
      console.log(props.dictionaryRoot, "**realHeight:", realHeight)
      // 赋值给 iframe
      val.style.height = `${realHeight + 10}px`

      setTimeout(() => {
        changedByThisCode.value = false
      }, 100)
    })

    // 监听整个 body 的所有变化
    mutationObserver.observe(doc.body, {
      childList: true,
      subtree: true,
      attributes: true,
      characterData: true
    })
  })
}, { immediate: true })

// 销毁时清理
onUnmounted(() => {
  mutationObserver?.disconnect()
})

// ================ 外层消息监听 ================
const messageListener = (e: MessageEvent) => {
  if (e.data?.iframeId !== iframeId.value) return

  if (e.data?.type === 'ENTRY_CLICK') {
    try {
      handleEntryClick(decodeURIComponent(e.data.entry))
    } catch {
      handleEntryClick(e.data.entry)
    }
  }
  else if (e.data?.type === 'SOUND_CLICK') {
    const soundUrl = `${baseUrl.value}/${e.data.sound}`
    const audio = new Audio(soundUrl)
    audio.currentTime = 0
    audio.play().catch(err => console.warn('播放失败', err))
  }
  else if (e.data?.type === 'LOCATION_CLICK') {
    const scrollContainer = document.querySelector('.word-detail') as HTMLElement
    if (!scrollContainer) return
    const iframeEl = document.getElementById(`dict-iframe-container-${props.dictionaryName}`) as HTMLElement
    if (!iframeEl) return
    const iframeTop = iframeEl.getBoundingClientRect().top - scrollContainer.getBoundingClientRect().top
    const targetScrollTop = scrollContainer.scrollTop + iframeTop + e.data.elementOffsetTop
    scrollContainer.scrollTo({
      top: targetScrollTop,
      behavior: 'instant'
    })
  }
  else if (e.data?.type === 'KEYDOWN') {
    emits('keydown', e.data)
  }
}

window.addEventListener('message', messageListener)

onUnmounted(() => {
  mutationObserver?.disconnect()
  window.removeEventListener('message', messageListener)
  if (iframeRef.value) {
    iframeRef.value.srcdoc = ''
  }
})
</script>
