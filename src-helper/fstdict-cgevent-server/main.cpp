#include <spdlog/cfg/env.h>
#include <thread>

#include "src/libs/logger.h"
#include "src/libs/selection_monitor.h"
#include "src/libs/websocket_server.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fcntl.h>
#include <iostream>
#include <pthread.h>
#include <signal.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

// 业务主函数（fork 之后才运行）
void run_main_business() {
  // =================== 你的业务 ===================
  LOG_INFO("=== 日志系统启动 ===");
  spdlog::cfg::load_env_levels();

  // 检查权限
  if (!ensureAccessibility()) { exit(1); }

  // 在独立线程中启动鼠标事件监听（选中文字功能）
  
  // WebSocket
  auto &ws_server = WebSocketServer::instance();
  std::thread ws_thread(&WebSocketServer::start_websocket_server, &ws_server);
  ws_thread.detach();
  start_mouse_event_listener();
}

int main(int argc, char *argv[]) {
  run_main_business();
  exit(EXIT_SUCCESS);
}