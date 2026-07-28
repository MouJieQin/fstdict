#ifndef WEBSOCKET_SERVER_H
#define WEBSOCKET_SERVER_H

#include <mutex>
#include <nlohmann/json.hpp>
#include <set>
#include <string>
#include <unordered_map>

#include "event_enum.hpp"
#include "safe_queue.hpp"
using json = nlohmann::json;

// 全局 WebSocket 客户端 fd
extern int g_ws_client;

class WebSocketServer {
  using string = std::string;

public:
  // 获取单例实例
  static WebSocketServer &instance();
  // 禁止拷贝和赋值
  WebSocketServer(const WebSocketServer &) = delete;
  WebSocketServer &operator=(const WebSocketServer &) = delete;

  void start_websocket_server();
  // 推送事件 JSON 到队列
  void push_event_json(const json &j);

  bool is_need_to_listen(EventType event) const;

private:
  WebSocketServer(size_t max_clients);
  // 发送原始数据
  static size_t ws_send_raw(int fd, const char *data, size_t len);

  // 发送 UTF-8 文本帧
  void ws_send_text(int fd, const string &text) const;
  // 发送 JSON
  void ws_send_json(int fd, const json &j) const;

  // 广播 JSON
  void broadcast_json(const json &j) const;

  // 解析帧
  static string ws_parse_frame(const char *data, size_t len);

  // 握手
  // base64 编码（用于生成 Sec-WebSocket-Accept）
  static string base64_encode(const unsigned char *buffer, size_t length);

  // 计算 WebSocket Accept 密钥
  static string compute_accept_key(const string &client_key);

  // 从请求头提取 Sec-WebSocket-Key
  static string get_ws_key(const char *data);

  static bool ws_handshake(int client);

  // 处理事件队列
  void handle_event_queue();
  // 处理客户端
  void handle_ws_client(int client);

private:
  size_t max_clients;
  std::vector<bool> client_connected;
  // 客户端互斥锁
  std::vector<std::unique_ptr<std::mutex>> client_mutexes;
  SafeQueue<json> event_queue;

  std::vector<std::set<size_t>> events_map_clients;
  std::vector<std::set<EventType>> clients_map_events;
  std::mutex events_map_mutex;
};

#endif