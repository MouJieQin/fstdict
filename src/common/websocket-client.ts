import { ref } from 'vue'
import {
    WS_RECONNECT_BASE_DELAY,
    WS_RECONNECT_MAX_DELAY,
    WS_RECONNECT_THRESHOLD,
} from '@/common/constants'

export const WebSocketStatus = {
    CONNECTING: 'connecting',
    OPEN: 'open',
    CLOSING: 'closing',
    CLOSED: 'closed',
    ERROR: 'error',
} as const

export type WebSocketStatusType = typeof WebSocketStatus[keyof typeof WebSocketStatus]

export abstract class WebSocketService {
    private socket: WebSocket | null = null
    public readonly status = ref<WebSocketStatusType>(WebSocketStatus.CLOSED)
    private reconnectTimer: ReturnType<typeof setTimeout> | null = null
    private reconnectAttempts = 0
    private readonly url: string
    private manuallyClosed = false

    protected onMessage: ((message: unknown) => void) | null = null

    constructor(url: string) {
        this.url = url
        this.connect()
    }

    private connect(): void {
        this.status.value = WebSocketStatus.CONNECTING
        this.socket = new WebSocket(this.url)

        this.socket.onopen = () => {
            this.status.value = WebSocketStatus.OPEN
            this.reconnectAttempts = 0
            this.clearReconnectTimer()
            this.handleOpen()
        }

        this.socket.onmessage = (event: MessageEvent<string>) => {
            try {
                const message = JSON.parse(event.data)
                this.handleMessage(message)
            } catch (err) {
                console.error('Failed to parse WebSocket message:', err)
            }
        }

        this.socket.onerror = (error: Event) => {
            this.status.value = WebSocketStatus.ERROR
            this.handleError(error)
            this.scheduleReconnect()
        }

        this.socket.onclose = (event: CloseEvent) => {
            this.status.value = WebSocketStatus.CLOSED
            this.handleClose(event)
            if (!this.manuallyClosed) {
                this.scheduleReconnect()
            }
        }
    }

    public send(message: unknown): void {
        if (this.socket?.readyState === WebSocket.OPEN) {
            this.socket.send(JSON.stringify(message))
        } else {
            console.warn('WebSocket is not connected; message dropped')
        }
    }

    public close(): void {
        this.manuallyClosed = true
        this.clearReconnectTimer()
        this.socket?.close()
    }

    private scheduleReconnect(): void {
        if (this.reconnectTimer) {
            this.clearReconnectTimer();
        }

        const delay = this.reconnectAttempts > WS_RECONNECT_THRESHOLD
            ? WS_RECONNECT_MAX_DELAY
            : WS_RECONNECT_BASE_DELAY

        this.reconnectTimer = setTimeout(() => {
            this.reconnectAttempts += 1
            this.connect()
        }, delay)
    }

    private clearReconnectTimer(): void {
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer)
            this.reconnectTimer = null
        }
    }

    protected handleOpen(): void {
        console.log('WebSocket connection established')
    }

    protected handleMessage(message: unknown): void {
        this.onMessage?.(message)
    }

    protected handleError(error: Event): void {
        console.error('WebSocket error:', error)
    }

    protected handleClose(event: CloseEvent): void {
        console.log(`WebSocket closed (code: ${event.code}, reason: ${event.reason})`)
    }

    public setMessageHandler(handler: (msg: unknown) => void): void {
        this.onMessage = handler
    }
}