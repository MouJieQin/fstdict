#include "websocket_server.h"
#include "http_client.h"
#include "logger.h"
#include <CommonCrypto/CommonDigest.h>
#include <arpa/inet.h>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <iomanip>
#include <iostream>
#include <netinet/in.h>
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

// 发送 PONG 控制帧（服务端回复客户端PING）
void WebSocketServer::ws_send_pong(int fd, const uint8_t *payload,
                                   size_t payload_len) const {
  std::lock_guard<std::mutex> lock(*client_mutexes[fd]);
  if (fd <= 0) return;

  uint8_t header[128];
  size_t hlen = 0;
  header[hlen++] = 0x8A; // FIN + PONG opcode(0xA)

  if (payload_len < 126) {
    header[hlen++] = static_cast<uint8_t>(payload_len);
  } else if (payload_len < 65536) {
    header[hlen++] = 126;
    header[hlen++] = static_cast<uint8_t>((payload_len >> 8) & 0xFF);
    header[hlen++] = static_cast<uint8_t>(payload_len & 0xFF);
  }
  // 一般ping载荷很短，无需实现127超长负载

  memcpy(header + hlen, payload, payload_len);
  hlen += payload_len;

  ws_send_raw(fd, reinterpret_cast<const char *>(header), hlen);
}

// 解析帧
string WebSocketServer::ws_parse_frame(int client, const char *data,
                                       size_t len) const {
  if (len < 6) return "";

  uint8_t fin = (data[0] >> 7) & 1;
  uint8_t opcode = data[0] & 0x0F;
  uint8_t mask = (data[1] >> 7) & 1;
  uint64_t plen = data[1] & 0x7F;

  // Calculate standard base dynamic offset lengths
  size_t payload_start = 2;
  if (plen == 126) {
    if (len < 4) return "";
    plen = ((unsigned char)data[2] << 8) | (unsigned char)data[3];
    payload_start += 2;
  } else if (plen == 127) {
    if (len < 10) return "";
    plen = 0;
    for (int i = 0; i < 8; i++) {
      plen = (plen << 8) | (unsigned char)data[2 + i];
    }
    payload_start += 8;
  }

  if (!mask) return "";
  if (len < payload_start + 4 + plen) return "";

  const uint8_t *mask_key = (const uint8_t *)data + payload_start;
  payload_start += 4;

  // Extract and unmask the frame payload
  string msg;
  msg.reserve(plen);
  for (size_t i = 0; i < plen; i++) {
    msg += data[payload_start + i] ^ mask_key[i % 4];
  }

  // -------------------------------------------------------------------------
  // ACTIONABLE PING/PONG INTERACTION HOOK
  // -------------------------------------------------------------------------
  if (opcode == 0x9) {
    // According to RFC-6455, a Pong MUST include the identical payload
    // sent within the triggering Ping frame.
    ws_send_pong(client, reinterpret_cast<const uint8_t *>(msg.data()),
                 msg.length());
    return ""; // Return empty string to seamlessly ignore this in the main
               // router loop
  }

  if (opcode == 0xA) {
    return ""; // Drop unidirectional client-side pongs
  }

  if (opcode == 0x8) {
    return "close"; // Request structural cleanup hook matching string
  }

  return msg;
}

static const char base64_table[] =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

std::string WebSocketServer::base64_encode(const unsigned char *data,
                                           size_t len) {
  std::string out;
  out.reserve(((len + 2) / 3) * 4);

  for (size_t i = 0; i < len; i += 3) {
    uint32_t block = data[i] << 16;
    if (i + 1 < len) block |= data[i + 1] << 8;
    if (i + 2 < len) block |= data[i + 2];

    out += base64_table[(block >> 18) & 0x3F];
    out += base64_table[(block >> 12) & 0x3F];
    if (i + 1 < len) {
      out += base64_table[(block >> 6) & 0x3F];
    } else {
      out += '=';
    }
    if (i + 2 < len) {
      out += base64_table[block & 0x3F];
    } else {
      out += '=';
    }
  }
  return out;
}

// 计算 WebSocket Accept 密钥（使用原生CC_SHA1替代OpenSSL SHA1）
std::string WebSocketServer::compute_accept_key(const std::string &client_key) {
  std::string guid = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
  std::string concat = client_key + guid;

  unsigned char hash[CC_SHA1_DIGEST_LENGTH];
  CC_SHA1(reinterpret_cast<const unsigned char *>(concat.c_str()),
          static_cast<CC_LONG>(concat.size()), hash);

  return base64_encode(hash, CC_SHA1_DIGEST_LENGTH);
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
    LOG_ERROR("[WebSocket] [{}] Client handshake failed", client);
    return;
  }
  LOG_INFO("[WebSocket] [{}] Client connected successfully", client);
  client_connected[client] = true;

  char buf[4096];
  while (true) {
    memset(buf, 0, sizeof(buf));

    // Leave 1 trailing byte free to guarantee a secure null-terminator string
    // slot
    ssize_t n = recv(client, buf, sizeof(buf) - 1, 0);
    if (n <= 0) { break; }

    // Explicitly null-terminate raw socket context buffer
    buf[n] = '\0';

    string msg = ws_parse_frame(client, buf, n);
    if (msg.empty()) { continue; }

    // Drop background network framework ping/pong heartbeat messages safely
    if (msg.find("keepalive") != string::npos ||
        msg.find("ping") != string::npos) {
      LOG_INFO("[WebSocket] [{}] Dropped control/ping keepalive payload frame",
               client);
      continue;
    }

    // Catch native close frames before they hit the json tokenizer stream
    if (msg == "close" || (msg.length() > 0 && msg[0] == '\x03')) {
      LOG_INFO("[WebSocket] [{}] Client requested connection tear-down",
               client);
      break;
    }

    LOG_INFO("[WebSocket] Received frame from client [{}]: {}", client, msg);

    json j;
    try {
      j = json::parse(msg);
    } catch (const json::exception &e) {
      LOG_ERROR("[WebSocket] [{}] JSON parsing failed: {}", client, e.what());
      continue;
    } catch (...) {
      LOG_ERROR(
          "[WebSocket] [{}] JSON parsing failed: Unknown exception caught",
          client);
      continue;
    }

    if (!j.contains("type")) {
      LOG_WARN("[WebSocket] [{}] Message rejected: Missing 'type' field",
               client);
    } else {
      std::string type = j["type"];
      if (type == "register_request") {
        if (!j.contains("data")) {
          LOG_WARN("[WebSocket] [{}] Registration rejected: Missing 'data' "
                   "payload object",
                   client);
        } else {
          const json &data = j["data"];
          if (!data.contains("event")) {
            LOG_WARN("[WebSocket] [{}] Registration rejected: Missing target "
                     "'event' key",
                     client);
          } else {
            std::string event = data["event"];
            auto type = EventTypeEnum::fromString(event);
            if (!type) {
              LOG_WARN("[WebSocket] [{}] Registration rejected: Unknown target "
                       "event code [{}]",
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
          LOG_WARN("[WebSocket] [{}] Unregistration rejected: Missing 'data' "
                   "payload object",
                   client);
        } else {
          const json &data = j["data"];
          if (!data.contains("event")) {
            LOG_WARN("[WebSocket] [{}] Unregistration rejected: Missing target "
                     "'event' key",
                     client);
          } else {
            std::string event = data["event"];
            auto type = EventTypeEnum::fromString(event);
            if (!type) {
              LOG_WARN("[WebSocket] [{}] Unregistration rejected: Unknown "
                       "target event code [{}]",
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
        LOG_WARN(
            "[WebSocket] [{}] Request dropped: Unknown event schema type [{}]",
            client, type);
      }
    }
  }

  // Graceful state cleanup sequences
  client_connected[client] = false;
  close(client);
  LOG_INFO("[WebSocket] [{}] Connection destroyed cleanly", client);
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
  std::string fstdict_api_url = "http://127.0.0.1:5959/api/connectcgevent";
  http_get_async(fstdict_api_url, 8, [](bool ok, const std::string &result) {
    if (ok) {
      LOG_INFO("[WebSocket] 成功通知 FSTDict 后端 CGEvent 监听服务已启动: {}",
               result);
    } else {
      LOG_ERROR("[WebSocket] 通知 FSTDict 后端 CGEvent 监听服务启动失败: {}",
                result);
    }
  });

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