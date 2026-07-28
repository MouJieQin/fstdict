#include "websocket_server.h"
#include "logger.h"
#include <arpa/inet.h>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <iomanip>
#include <iostream>
#include <netinet/in.h>
#include <openssl/bio.h>
#include <openssl/buffer.h>
#include <openssl/evp.h>
#include <openssl/sha.h>
#include <sstream>
#include <sys/socket.h>
#include <thread>
#include <unistd.h>

using namespace std;

#define WS_PORT 5995
int g_ws_client = -1;
mutex ws_mutex;

WebSocketServer &WebSocketServer::instance() {
  static WebSocketServer ws_server(100);
  return ws_server;
}

WebSocketServer::WebSocketServer(size_t max_clients)
    : max_clients(max_clients), client_connected(max_clients, false),
      client_mutexes(max_clients), event_queue(),
      events_map_clients(getEnumNames(EventType{}).size()),
      clients_map_events(max_clients) {
  for (size_t i = 0; i < max_clients; i++) {
    client_mutexes[i] = make_unique<mutex>();
  }
}

void WebSocketServer::push_event_json(const json &j) { event_queue.push(j); }

bool WebSocketServer::is_need_to_listen(EventType event) const {
  return !events_map_clients[size_t(event)].empty();
}

// 发送原始数据
size_t WebSocketServer::ws_send_raw(int fd, const char *data, size_t len) {
  return send(fd, data, len, 0);
}

// 发送 UTF-8 文本帧
void WebSocketServer::ws_send_text(int fd, const string &text) const {
  lock_guard<mutex> lock(*client_mutexes[fd]);
  if (fd <= 0) return;

  uint8_t header[10];
  size_t hlen = 0;
  size_t plen = text.size();

  header[hlen++] = 0x81;
  if (plen < 126) {
    header[hlen++] = plen;
  } else if (plen < 65536) {
    header[hlen++] = 126;
    header[hlen++] = (plen >> 8) & 0xFF;
    header[hlen++] = plen & 0xFF;
  }

  ws_send_raw(fd, (char *)header, hlen);
  ws_send_raw(fd, text.c_str(), plen);
}

// 发送 JSON
void WebSocketServer::ws_send_json(int fd, const json &j) const {
  ws_send_text(fd, j.dump());
}

void WebSocketServer::broadcast_json(const json &j) const {
  for (size_t i = 0; i < max_clients; i++) {
    if (client_connected[i]) { ws_send_json(i, j); }
  }
}

// 解析帧
string WebSocketServer::ws_parse_frame(const char *data, size_t len) {
  if (len < 6) return "";
  uint8_t fin = (data[0] >> 7) & 1;
  uint8_t opcode = data[0] & 0x0F;
  uint8_t mask = (data[1] >> 7) & 1;
  uint8_t plen = data[1] & 0x7F;

  size_t payload_start = 2;
  if (plen == 126) payload_start += 2;
  if (!mask) return "";

  const uint8_t *mask_key = (const uint8_t *)data + payload_start;
  payload_start += 4;

  string msg;
  for (size_t i = 0; i < plen; i++) {
    msg += data[payload_start + i] ^ mask_key[i % 4];
  }
  return msg;
}

// 握手
// base64 编码（用于生成 Sec-WebSocket-Accept）
string WebSocketServer::base64_encode(const unsigned char *buffer,
                                      size_t length) {
  BIO *bio, *b64;
  BUF_MEM *buf;

  b64 = BIO_new(BIO_f_base64());
  bio = BIO_new(BIO_s_mem());
  bio = BIO_push(b64, bio);

  BIO_set_flags(bio, BIO_FLAGS_BASE64_NO_NL);
  BIO_write(bio, buffer, length);
  BIO_flush(bio);
  BIO_get_mem_ptr(bio, &buf);

  string ret(buf->data, buf->length);
  BIO_free_all(bio);
  return ret;
}

// 计算 WebSocket Accept 密钥
string WebSocketServer::compute_accept_key(const string &client_key) {
  string guid = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
  string concat = client_key + guid;

  unsigned char hash[SHA_DIGEST_LENGTH];
  SHA1((const unsigned char *)concat.c_str(), concat.size(), hash);

  return base64_encode(hash, SHA_DIGEST_LENGTH);
}

// 从请求头提取 Sec-WebSocket-Key
string WebSocketServer::get_ws_key(const char *data) {
  string s(data);
  size_t pos = s.find("Sec-WebSocket-Key: ");
  if (pos == string::npos) return "";

  pos += 19;
  size_t end = s.find("\r\n", pos);
  return s.substr(pos, end - pos);
}

// ✅ 修复后的正确握手
bool WebSocketServer::ws_handshake(int client) {
  char buf[1024];
  ssize_t n = recv(client, buf, sizeof(buf) - 1, 0);
  if (n <= 0) return false;
  buf[n] = 0;

  string key = get_ws_key(buf);
  if (key.empty()) return false;

  string accept_key = compute_accept_key(key);

  // 标准握手响应
  stringstream resp;
  resp << "HTTP/1.1 101 Switching Protocols\r\n";
  resp << "Upgrade: websocket\r\n";
  resp << "Connection: Upgrade\r\n";
  resp << "Sec-WebSocket-Accept: " << accept_key << "\r\n";
  resp << "\r\n";

  string response = resp.str();
  send(client, response.c_str(), response.size(), 0);
  return true;
}

// 处理客户端
void WebSocketServer::handle_ws_client(int client) {

  if (!ws_handshake(client)) {
    close(client);
    LOG_ERROR("[WebSocket] [{}] 客户端握手失败", client);
    return;
  }
  LOG_INFO("[WebSocket] [{}] 客户端已连接", client);
  client_connected[client] = true;

  char buf[2048];
  while (true) {
    ssize_t n = recv(client, buf, sizeof(buf), 0);
    if (n <= 0) { break; }

    string msg = ws_parse_frame(buf, n);
    if (msg.empty()) { continue; }
    LOG_INFO("[WebSocket] 收到客户端 [{}] 消息: {}", client, msg);

    // 处理 JSON 消息
    json j;
    try {
      j = json::parse(msg);
    } catch (const json::exception &e) {
      LOG_ERROR("[WebSocket] [{}] 解析 JSON 消息失败: {}", client, e.what());
      continue;
    } catch (...) {
      LOG_ERROR("[WebSocket] [{}] 解析 JSON 消息失败: 未知异常", client);
      continue;
    }

    if (!j.contains("type")) {
      LOG_WARN("[WebSocket] [{}] 解析 JSON 消息失败: 缺少 type 字段", client);
    } else {
      std::string type = j["type"];
      if (type == "register_request") {
        if (!j.contains("data")) {
          LOG_WARN("[WebSocket] [{}] 解析 JSON 消息失败: 缺少 data 字段",
                   client);
        } else {
          const json &data = j["data"];
          if (!data.contains("event")) {
            LOG_WARN("[WebSocket] [{}] 解析 JSON 消息失败: 缺少 event 字段",
                     client);
          } else {
            std::string event = data["event"];
            auto type = EventTypeEnum::fromString(event);
            if (!type) {
              LOG_WARN("[WebSocket] [{}] 解析 JSON 消息失败: 未知 event: {}",
                       client, event);
            } else {
              size_t event_index = size_t(type.value());
              json resp = {{"type", "register_response"},
                           {"data", {{"event", event}, {"success", true}}}};
              ws_send_json(client, resp);
              std::lock_guard<std::mutex> lock(events_map_mutex);
              events_map_clients[event_index].insert(client);
              clients_map_events[client].insert(type.value());
            }
          }
        }
      } else if (type == "unregister_request") {
        if (!j.contains("data")) {
          LOG_WARN("[WebSocket] [{}] 解析 JSON 消息失败: 缺少 data 字段",
                   client);
        } else {
          const json &data = j["data"];
          if (!data.contains("event")) {
            LOG_WARN("[WebSocket] [{}] 解析 JSON 消息失败: 缺少 event 字段",
                     client);
          } else {
            std::string event = data["event"];
            auto type = EventTypeEnum::fromString(event);
            if (!type) {
              LOG_WARN("[WebSocket] [{}] 解析 JSON 消息失败: 未知 event: [{}]",
                       client, event);
            } else {
              size_t event_index = size_t(type.value());
              json resp = {{"type", "unregister_response"},
                           {"data", {{"event", event}, {"success", true}}}};
              ws_send_json(client, resp);
              std::lock_guard<std::mutex> lock(events_map_mutex);
              events_map_clients[event_index].erase(client);
              clients_map_events[client].erase(type.value());
            }
          }
        }
      } else {
        LOG_WARN("[WebSocket] [{}] 解析 JSON 消息失败: 未知 type: {}", client,
                 type);
      }
    }
    LOG_INFO("[WebSocket] 发送客户端 [{}] 消息: {}", client, j.dump());
  }
  client_connected[client] = false;
  close(client);
  LOG_INFO("[WebSocket] [{}] 客户端断开连接", client);
  std::lock_guard<std::mutex> lock(events_map_mutex);
  for (auto &event : clients_map_events[client]) {
    events_map_clients[size_t(event)].erase(client);
  }
  clients_map_events[client].clear();
}

// ================= 测试 =================
// 线程B：消费者（处理数据，不轮询、不sleep）
void WebSocketServer::handle_event_queue() {
  while (true) {
    // 重点：没有 while(!empty())，没有 sleep！
    // 队列为空时自动休眠，不消耗CPU
    json event_data = event_queue.wait_and_pop();
    std::string type = event_data["type"];
    if (type == "CGEvent") {
      std::lock_guard<std::mutex> lock(events_map_mutex);
      auto event_type =
          EventTypeEnum::fromString(std::string(event_data["data"]["type"]));
      size_t event_index = size_t(event_type.value());
      for (auto &client : events_map_clients[event_index]) {
        ws_send_json(client, event_data);
      }
    } else {
      LOG_WARN("未处理事件类型: {}", type);
    }
  }
}

// ========== 修复 bind 编译错误的核心代码 ==========
void WebSocketServer::start_websocket_server() {
  int server_fd = socket(AF_INET, SOCK_STREAM, 0);
  if (server_fd < 0) {
    perror("socket failed");
    LOG_ERROR("创建 WebSocket 服务器失败！");
    return;
  }

  int opt = 1;
  setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

  struct sockaddr_in address;
  address.sin_family = AF_INET;
  address.sin_addr.s_addr = INADDR_ANY;
  address.sin_port = htons(WS_PORT);

  socklen_t addrlen = sizeof(address);

  // 🔥 修复 macOS bind 报错：强制类型转换 + 正确参数
  if (::bind(server_fd, (struct sockaddr *)&address, addrlen) < 0) {
    perror("bind failed");
    close(server_fd);
    LOG_ERROR("绑定 WebSocket 服务器失败！");
    return;
  }

  if (listen(server_fd, 1) < 0) {
    perror("listen failed");
    close(server_fd);
    LOG_ERROR("监听 WebSocket 服务器失败！");
    return;
  }

  LOG_INFO("[WebSocket] 运行在 ws://localhost:{}", WS_PORT);

  auto &ws_server = WebSocketServer::instance();
  thread(&WebSocketServer::handle_event_queue, &ws_server).detach();
  while (true) {
    int client_fd = accept(server_fd, (struct sockaddr *)&address, &addrlen);
    if (client_fd >= 0) {
      if (size_t(client_fd) >= max_clients) {
        close(client_fd);
        LOG_WARN("[WebSocket] [{}] accept failed: 客户端数量[{}] 已达上限, "
                 "已拒绝连接",
                 client_fd, max_clients);
      } else {
        LOG_INFO("[WebSocket] accept success: [{}]", client_fd);
        thread(&WebSocketServer::handle_ws_client, &ws_server, client_fd)
            .detach();
      }
    } else {
      LOG_ERROR("[WebSocket] accept failed: {}", strerror(errno));
    }
  }
}