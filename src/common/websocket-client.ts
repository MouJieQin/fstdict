import { ref } from 'vue';

// WebSocket 状态枚举
const WebSocketStatus = {
    CONNECTING: 'connecting',
    OPEN: 'open',
    CLOSING: 'closing',
    CLOSED: 'closed',
    ERROR: 'error'
} as const;

// // 自定义消息类型，根据实际后端协议调整
// type WebSocketMessage = {
//     type: string; // 消息类型，如 'chat'/'notification' 等
//     data: any;    // 消息内容，可细化为具体接口
// };

class WebSocketService {
    private socket: WebSocket | null = null;
    private status = ref<typeof WebSocketStatus[keyof typeof WebSocketStatus]>(WebSocketStatus.CLOSED);
    private reconnectInterval: number | null = null; // 重连定时器
    private reconnectCount: number = 0;
    private url: string; // WebSocket 服务器地址
    private isClosedByUser: boolean = false;

    constructor(url: string) {
        this.url = url;
        this.init();
    }

    // 初始化连接
    private init() {
        this.status.value = WebSocketStatus.CONNECTING;
        this.socket = new WebSocket(this.url);

        this.socket.onopen = () => {
            this.status.value = WebSocketStatus.OPEN;
            this.clearReconnect(); // 连接成功则清除重连定时器
            this.handleOpen();
        };

        this.socket.onmessage = (event: MessageEvent) => {
            const message: {} = JSON.parse(event.data);
            this.handleMessage(message);
        };

        this.socket.onerror = (error: Event) => {
            this.status.value = WebSocketStatus.ERROR;
            this.handleError(error);
            this.reconnect(); // 出错尝试重连
        };

        this.socket.onclose = (event: CloseEvent) => {
            this.status.value = WebSocketStatus.CLOSED;
            this.handleClose(event);
            this.reconnect(); // 关闭后尝试重连（根据需求调整，如用户主动关闭则不重连）
        };
    }

    // 发送消息
    send(message: {}) {
        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            this.socket.send(JSON.stringify(message));
        } else {
            console.warn('WebSocket 未连接或正在连接中，无法发送消息');
        }
    }

    // 主动关闭连接
    close() {
        this.isClosedByUser = true;
        if (this.socket) {
            this.socket.close();
            this.clearReconnect();
        }
    }

    // 重连逻辑
    private reconnect() {
        if (this.isClosedByUser) return;
        if (this.reconnectInterval) return;
        this.reconnectInterval = window.setInterval(() => {
            this.init();
            this.reconnectCount += 1;
        }, this.reconnectCount > 10 ? 5000 : 500); // 5 秒重连一次，可配置
    }

    private clearReconnect() {
        this.reconnectCount = 0;
        if (this.reconnectInterval) {
            window.clearInterval(this.reconnectInterval);
            this.reconnectInterval = null;
        }
    }

    // 以下为事件处理钩子，可被子类或外部重写
    protected handleOpen() {
        console.log('WebSocket 连接已建立');
    }

    handleMessage(message: {}) {
        console.log('收到 WebSocket 消息:', message);
        // 根据 message.type 分发不同业务逻辑，如：
        // if (message.type === 'chat') this.handleChatMessage(message.data);
    }

    protected handleError(error: Event) {
        console.error('WebSocket 错误:', error);
    }

    protected handleClose(event: CloseEvent) {
        console.log('WebSocket 连接关闭，代码:', event.code, '原因:', event.reason);
    }

    // 获取当前连接状态
    getStatus() {
        return this.status;
    }
}

// 导出单例或工厂函数，根据项目需求选择
let webSocketInstance: WebSocketService | null = null;
export function useWebSocket(url: string) {
    webSocketInstance = new WebSocketService(url);
    return webSocketInstance;
}
export { WebSocketService }