#include "websocket_server.h"
#include "http_client.h"
#include "logger.h"
#include "websocket_frame.h"

#include <arpa/inet.h>
#include <cerrno>
#include <cstring>
#include <netinet/in.h>
#include <sys/socket.h>
#include <thread>
#include <unistd.h>

WebSocketServer &WebSocketServer::instance() {
  static WebSocketServer server;
  return server;
}

WebSocketServer::WebSocketServer()
    : m_eventToClients(static_cast<size_t>(EventType::kHandlerTextSelection) +
                       1) {}

void WebSocketServer::pushEvent(const json &event) { m_eventQueue.push(event); }

bool WebSocketServer::isEventSubscribed(EventType event) const {
  std::lock_guard<std::mutex> lock(m_subscriptionMutex);
  size_t idx = static_cast<size_t>(event);
  return idx < m_eventToClients.size() && !m_eventToClients[idx].empty();
}

ssize_t WebSocketServer::sendRaw(int fd, const char *data, size_t len) {
  return send(fd, data, len, 0);
}

void WebSocketServer::sendText(int clientFd, const std::string &text) const {
  std::lock_guard<std::mutex> clientLock(*m_clients.at(clientFd).sendMutex);

  auto frame = WebSocketFrame::encodeTextFrame(text);
  sendRaw(clientFd, reinterpret_cast<const char *>(frame.data()), frame.size());
}

void WebSocketServer::sendJson(int clientFd, const json &j) const {
  sendText(clientFd, j.dump());
}

void WebSocketServer::sendPong(int clientFd, const uint8_t *payload,
                               size_t payloadLen) const {
  std::lock_guard<std::mutex> clientLock(*m_clients.at(clientFd).sendMutex);

  auto frame = WebSocketFrame::encodePongFrame(payload, payloadLen);
  sendRaw(clientFd, reinterpret_cast<const char *>(frame.data()), frame.size());
}

bool WebSocketServer::performHandshake(int clientFd) {
  char buffer[1024];
  ssize_t n = recv(clientFd, buffer, sizeof(buffer) - 1, 0);

  if (n <= 0) return false;
  buffer[n] = '\0';

  std::string key = WebSocketFrame::extractWsKey(buffer);
  if (key.empty()) return false;

  std::string acceptKey = WebSocketFrame::computeAcceptKey(key);
  std::string response = WebSocketFrame::buildHandshakeResponse(acceptKey);

  sendRaw(clientFd, response.c_str(), response.size());
  return true;
}

void WebSocketServer::subscribeEvent(int clientFd, EventType event) {
  std::lock_guard<std::mutex> subLock(m_subscriptionMutex);
  size_t eventIdx = static_cast<size_t>(event);

  if (eventIdx < m_eventToClients.size()) {
    m_eventToClients[eventIdx].insert(clientFd);
  }

  std::lock_guard<std::mutex> clientLock(m_clientsMutex);
  if (m_clients.count(clientFd)) {
    m_clients[clientFd].subscribedEvents.insert(event);
  }
}

void WebSocketServer::unsubscribeEvent(int clientFd, EventType event) {
  std::lock_guard<std::mutex> subLock(m_subscriptionMutex);
  size_t eventIdx = static_cast<size_t>(event);

  if (eventIdx < m_eventToClients.size()) {
    m_eventToClients[eventIdx].erase(clientFd);
  }

  std::lock_guard<std::mutex> clientLock(m_clientsMutex);
  if (m_clients.count(clientFd)) {
    m_clients[clientFd].subscribedEvents.erase(event);
  }
}

void WebSocketServer::cleanupClient(int clientFd) {
  // Remove from subscription maps
  {
    std::lock_guard<std::mutex> subLock(m_subscriptionMutex);
    for (size_t i = 0; i < m_eventToClients.size(); ++i) {
      m_eventToClients[i].erase(clientFd);
    }
  }

  // Remove from client map
  {
    std::lock_guard<std::mutex> clientLock(m_clientsMutex);
    m_clients.erase(clientFd);
  }

  close(clientFd);
  LOG_INFO("[WebSocket] Client [{}] disconnected and cleaned up", clientFd);
}

void WebSocketServer::handleClient(int clientFd) {
  if (!performHandshake(clientFd)) {
    close(clientFd);
    LOG_ERROR("[WebSocket] Client [{}] handshake failed", clientFd);
    return;
  }

  // Initialize client state
  {
    std::lock_guard<std::mutex> lock(m_clientsMutex);
    ClientState state;
    state.connected = true;
    state.sendMutex = std::make_unique<std::mutex>();
    m_clients[clientFd] = std::move(state);
  }

  LOG_INFO("[WebSocket] Client [{}] connected", clientFd);

  char buffer[4096];
  while (true) {
    memset(buffer, 0, sizeof(buffer));
    ssize_t n = recv(clientFd, buffer, sizeof(buffer) - 1, 0);

    if (n <= 0) {
      LOG_INFO("[WebSocket] Client [{}] connection closed", clientFd);
      break;
    }

    std::string payload;
    uint8_t opcode = 0;

    if (!WebSocketFrame::decodeFrame(buffer, static_cast<size_t>(n), payload,
                                     opcode)) {
      LOG_WARN("[WebSocket] Client [{}] sent invalid frame", clientFd);
      continue;
    }

    // Handle control frames
    if (opcode == 0x9) { // PING
      sendPong(clientFd, reinterpret_cast<const uint8_t *>(payload.data()),
               payload.size());
      continue;
    }
    if (opcode == 0xA) { // PONG (ignore)
      continue;
    }
    if (opcode == 0x8) { // CLOSE
      LOG_INFO("[WebSocket] Client [{}] requested close", clientFd);
      break;
    }

    // Handle text frames (JSON messages)
    if (opcode == 0x1) {
      if (payload.empty()) continue;

      // Skip simple keepalive messages
      if (payload.find("keepalive") != std::string::npos ||
          payload.find("ping") != std::string::npos) {
        LOG_DEBUG("[WebSocket] Client [{}] keepalive", clientFd);
        continue;
      }

      LOG_INFO("[WebSocket] Received from [{}]: {}", clientFd, payload);

      try {
        json msg = json::parse(payload);

        if (!msg.contains("type")) {
          LOG_WARN("[WebSocket] Client [{}] message missing 'type' field",
                   clientFd);
          continue;
        }

        std::string type = msg["type"];

        if (type == "register_request") {
          if (!msg.contains("data") || !msg["data"].contains("event")) {
            LOG_WARN("[WebSocket] Invalid register request from [{}]",
                     clientFd);
            continue;
          }

          std::string eventName = msg["data"]["event"];
          auto eventOpt = EventTypeUtil::fromString(eventName);

          if (!eventOpt.has_value()) {
            LOG_WARN("[WebSocket] Unknown event type '{}' from [{}]", eventName,
                     clientFd);
            continue;
          }

          subscribeEvent(clientFd, eventOpt.value());

          json resp = {{"type", "register_response"},
                       {"data", {{"event", eventName}, {"success", true}}}};
          sendJson(clientFd, resp);
          LOG_INFO("[WebSocket] Client [{}] subscribed to {}", clientFd,
                   eventName);
        } else if (type == "unregister_request") {
          if (!msg.contains("data") || !msg["data"].contains("event")) {
            LOG_WARN("[WebSocket] Invalid unregister request from [{}]",
                     clientFd);
            continue;
          }

          std::string eventName = msg["data"]["event"];
          auto eventOpt = EventTypeUtil::fromString(eventName);

          if (!eventOpt.has_value()) {
            LOG_WARN("[WebSocket] Unknown event type '{}' from [{}]", eventName,
                     clientFd);
            continue;
          }

          unsubscribeEvent(clientFd, eventOpt.value());

          json resp = {{"type", "unregister_response"},
                       {"data", {{"event", eventName}, {"success", true}}}};
          sendJson(clientFd, resp);
          LOG_INFO("[WebSocket] Client [{}] unsubscribed from {}", clientFd,
                   eventName);
        } else {
          LOG_WARN("[WebSocket] Unknown message type '{}' from [{}]", type,
                   clientFd);
        }

      } catch (const json::exception &e) {
        LOG_ERROR("[WebSocket] JSON parse error from [{}]: {}", clientFd,
                  e.what());
      } catch (...) {
        LOG_ERROR("[WebSocket] Unknown error processing message from [{}]",
                  clientFd);
      }
    }
  }

  cleanupClient(clientFd);
}

void WebSocketServer::eventQueueWorker() {
  while (true) {
    json event = m_eventQueue.waitAndPop();

    std::string eventCategory = event.value("type", "");
    if (eventCategory != "CGEvent") {
      LOG_WARN("Ignoring unknown event category: {}", eventCategory);
      continue;
    }

    std::string eventTypeStr = event["data"]["type"];
    auto eventTypeOpt = EventTypeUtil::fromString(eventTypeStr);

    if (!eventTypeOpt.has_value()) {
      LOG_WARN("Ignoring unknown event type: {}", eventTypeStr);
      continue;
    }

    size_t eventIdx = static_cast<size_t>(eventTypeOpt.value());
    std::set<int> targetClients;

    // Snapshot subscribers under lock
    {
      std::lock_guard<std::mutex> lock(m_subscriptionMutex);
      if (eventIdx < m_eventToClients.size()) {
        targetClients = m_eventToClients[eventIdx];
      }
    }

    // Send outside the lock to reduce contention
    for (int clientFd : targetClients) {
      try {
        sendJson(clientFd, event);
      } catch (...) {
        LOG_WARN("[WebSocket] Failed to send event to client [{}]", clientFd);
      }
    }
  }
}

void WebSocketServer::startServer() {
  int serverFd = socket(AF_INET, SOCK_STREAM, 0);
  if (serverFd < 0) {
    LOG_CRITICAL("Failed to create server socket: {}", strerror(errno));
    return;
  }

  int opt = 1;
  setsockopt(serverFd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

  struct sockaddr_in address{};
  address.sin_family = AF_INET;
  address.sin_addr.s_addr = INADDR_ANY;
  address.sin_port = htons(WS_PORT);
  socklen_t addrLen = sizeof(address);

  if (::bind(serverFd, reinterpret_cast<struct sockaddr *>(&address), addrLen) <
      0) {
    LOG_CRITICAL("Failed to bind port {}: {}", WS_PORT, strerror(errno));
    close(serverFd);
    return;
  }

  if (listen(serverFd, SOMAXCONN) < 0) {
    LOG_CRITICAL("Failed to listen on socket: {}", strerror(errno));
    close(serverFd);
    return;
  }

  LOG_INFO("[WebSocket] Server listening on ws://127.0.0.1:{}", WS_PORT);

  // Notify backend that CGEvent service is ready
  const std::string backendUrl = "http://127.0.0.1:5959/api/connectcgevent";
  httpGetAsync(backendUrl, 8, [](bool ok, const std::string &result) {
    if (ok) {
      LOG_INFO("[WebSocket] Successfully notified backend service: {}", result);
    } else {
      LOG_WARN("[WebSocket] Could not reach backend service: {}", result);
    }
  });

  // Start event broadcast worker thread
  std::thread(&WebSocketServer::eventQueueWorker, this).detach();

  // Accept loop
  while (true) {
    int clientFd = accept(
        serverFd, reinterpret_cast<struct sockaddr *>(&address), &addrLen);

    if (clientFd >= 0) {
      // Check client limit
      {
        std::lock_guard<std::mutex> lock(m_clientsMutex);
        if (m_clients.size() >= MAX_CLIENTS) {
          close(clientFd);
          LOG_WARN("[WebSocket] Rejected client [{}]: max clients ({}) reached",
                   clientFd, MAX_CLIENTS);
          continue;
        }
      }

      LOG_INFO("[WebSocket] New client connected: [{}]", clientFd);
      std::thread(&WebSocketServer::handleClient, this, clientFd).detach();
    } else {
      LOG_ERROR("[WebSocket] accept() failed: {}", strerror(errno));
    }
  }
}
