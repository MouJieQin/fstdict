#pragma once

#include <atomic>
#include <mutex>
#include <nlohmann/json.hpp>
#include <set>
#include <string>
#include <unordered_map>
#include <vector>

#include "event_enum.hpp"
#include "safe_queue.hpp"

using json = nlohmann::json;

/// Thread-safe WebSocket server with event subscription system
class WebSocketServer {
public:
  /// Get singleton instance
  static WebSocketServer &instance();

  WebSocketServer(const WebSocketServer &) = delete;
  WebSocketServer &operator=(const WebSocketServer &) = delete;

  /// Start the server (blocks on calling thread)
  void startServer();

  /// Push an event JSON into the broadcast queue
  void pushEvent(const json &event);

  /// Check if any client is subscribed to the given event
  bool isEventSubscribed(EventType event) const;

private:
  struct ClientState {
    bool connected = false;
    std::unique_ptr<std::mutex> sendMutex;
    std::set<EventType> subscribedEvents;
  };

  WebSocketServer();

  /// Send raw bytes to a client (thread-safe per-client)
  static ssize_t sendRaw(int fd, const char *data, size_t len);

  /// Send text frame to a single client
  void sendText(int clientFd, const std::string &text) const;

  /// Send JSON to a single client
  void sendJson(int clientFd, const json &j) const;

  /// Send PONG frame to a client
  void sendPong(int clientFd, const uint8_t *payload, size_t payloadLen) const;

  /// Perform WebSocket opening handshake
  static bool performHandshake(int clientFd);

  /// Handle full lifecycle of a single client connection
  void handleClient(int clientFd);

  /// Background worker that broadcasts events from the queue
  void eventQueueWorker();

  /// Register client for an event type
  void subscribeEvent(int clientFd, EventType event);

  /// Unregister client from an event type
  void unsubscribeEvent(int clientFd, EventType event);

  /// Clean up all client state on disconnect
  void cleanupClient(int clientFd);

private:
  static constexpr uint16_t WS_PORT = 5995;
  static constexpr size_t MAX_CLIENTS = 100;

  SafeQueue<json> m_eventQueue;

  mutable std::mutex m_clientsMutex;
  std::unordered_map<int, ClientState> m_clients;

  mutable std::mutex m_subscriptionMutex;
  std::vector<std::set<int>> m_eventToClients; // index = EventType ordinal
};
