#include <cstdlib>
#include <spdlog/cfg/env.h>
#include <thread>

#include "src/libs/accessibility_manager.h"
#include "src/libs/logger.h"
#include "src/libs/selection_monitor.h"
#include "src/libs/websocket_server.h"

/// Main business logic entry point
void runMainBusiness() {
  LOG_INFO("=== FstDict CGEvent Server Starting ===");

  // Load log level from environment variable (SPDLOG_LEVEL)
  spdlog::cfg::load_env_levels();

  // Verify accessibility permissions before starting
  if (!ensureAccessibilityPermissions()) {
    LOG_CRITICAL("Cannot start without accessibility permissions. Exiting.");
    exit(EXIT_FAILURE);
  }

  // Start WebSocket server in background thread
  auto &wsServer = WebSocketServer::instance();
  std::thread wsThread(&WebSocketServer::startServer, &wsServer);
  wsThread.detach();

  // Start mouse event listener (blocks current thread)
  startMouseEventListener();
}

int main(int argc, char *argv[]) {
  runMainBusiness();
  return EXIT_SUCCESS;
}
