import { onMounted, onUnmounted, ref } from 'vue'

interface EntryClickEvent {
    type: 'ENTRY_CLICK'
    iframeId: string
    entry: string
}

interface SoundClickEvent {
    type: 'SOUND_CLICK'
    iframeId: string
    sound: string
}

interface LocationClickEvent {
    type: 'LOCATION_CLICK'
    iframeId: string
    elementOffsetTop: number
}

interface KeydownEvent {
    type: 'KEYDOWN'
    iframeId: string
    key: string
    code: string
    ctrlKey: boolean
    shiftKey: boolean
    altKey: boolean
    metaKey: boolean
}

type IframeMessage =
    | EntryClickEvent
    | SoundClickEvent
    | LocationClickEvent
    | KeydownEvent

interface MessageHandlers {
    onEntryClick?: (entry: string, iframeId: string) => void
    onSoundClick?: (sound: string, iframeId: string) => void
    onLocationClick?: (offsetTop: number, iframeId: string) => void
    onKeydown?: (event: Omit<KeydownEvent, 'type'>) => void
}

export function useIframeMessaging(handlers: MessageHandlers) {
    const lastKeydownEvent = ref<Omit<KeydownEvent, 'type'> | null>(null)

    const handleMessage = (e: MessageEvent): void => {
        const data = e.data as IframeMessage
        if (!data?.type || !data.iframeId) return

        switch (data.type) {
            case 'ENTRY_CLICK':
                try {
                    handlers.onEntryClick?.(decodeURIComponent(data.entry), data.iframeId)
                } catch {
                    handlers.onEntryClick?.(data.entry, data.iframeId)
                }
                break

            case 'SOUND_CLICK':
                handlers.onSoundClick?.(data.sound, data.iframeId)
                break

            case 'LOCATION_CLICK':
                handlers.onLocationClick?.(data.elementOffsetTop, data.iframeId)
                break

            case 'KEYDOWN':
                const keydownData = {
                    key: data.key,
                    code: data.code,
                    ctrlKey: data.ctrlKey,
                    shiftKey: data.shiftKey,
                    altKey: data.altKey,
                    metaKey: data.metaKey,
                    iframeId: data.iframeId,
                }
                lastKeydownEvent.value = keydownData
                handlers.onKeydown?.(keydownData)
                break
        }
    }

    onMounted(() => {
        window.addEventListener('message', handleMessage)
    })

    onUnmounted(() => {
        window.removeEventListener('message', handleMessage)
    })

    return {
        lastKeydownEvent,
    }
}