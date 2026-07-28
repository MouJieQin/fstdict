#include <spdlog/cfg/env.h>
#include <thread>

#include "src/CGEventHandler.h"
#include "src/libs/logger.h"
#include "src/libs/selection_monitor.h"
#include "src/libs/shortcut_runner.h"
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

// ====================== 配置项 ======================
#define LOG_FILE "/tmp/daemon.log"
#define PID_FILE "/tmp/my_daemon.pid"
// ====================================================

static int g_log_fd = -1;

// 优雅退出
void graceful_exit(int sig) {
  if (g_log_fd >= 0) { LOG_INFO("收到退出信号，关闭..."); }
  LOG_INFO("停止事件循环...");
  // 停止主程序的 RunLoop，安全退出
  CFRunLoopStop(CFRunLoopGetCurrent());
  unlink(PID_FILE);
  if (g_log_fd >= 0) close(g_log_fd);
  exit(EXIT_SUCCESS);
}

void setup_signal() {
  signal(SIGTERM, graceful_exit);
  signal(SIGINT, graceful_exit);
}

// 创建 PID 防止重复启动
int create_pid_file() {
  int fd = open(PID_FILE, O_RDONLY);
  if (fd >= 0) {
    close(fd);
    return -1;
  }
  fd = open(PID_FILE, O_CREAT | O_WRONLY, 0644);
  if (fd < 0) return -1;
  char pid_str[32];
  snprintf(pid_str, sizeof(pid_str), "%d", getpid());
  write(fd, pid_str, strlen(pid_str));
  close(fd);
  return 0;
}

// 守护进程初始化（必须在所有业务之前！）
void init_daemon_early() {
  // 双 fork
  pid_t pid = fork();
  if (pid < 0) exit(EXIT_FAILURE);
  if (pid > 0) exit(EXIT_SUCCESS);

  setsid();

  pid = fork();
  if (pid < 0) exit(EXIT_FAILURE);
  if (pid > 0) exit(EXIT_SUCCESS);

  // chdir("/");
  umask(0);

  // 重定向输出
  g_log_fd = open(LOG_FILE, O_CREAT | O_WRONLY | O_APPEND, 0644);
  dup2(g_log_fd, STDOUT_FILENO);
  dup2(g_log_fd, STDERR_FILENO);
  close(STDIN_FILENO);

  // PID 文件
  if (create_pid_file() < 0) {
    dprintf(g_log_fd, "[ERROR] 进程已在运行！\n");
    exit(EXIT_FAILURE);
  }

  setup_signal();
  dprintf(g_log_fd, "[INFO] 守护进程初始化完成 PID=%d\n", getpid());
}

// 业务主函数（fork 之后才运行）
void run_main_business() {
  // =================== 你的业务 ===================
  LOG_INFO("=== 日志系统启动 ===");
  spdlog::cfg::load_env_levels();

  // 检查权限
  if (!ensureAccessibility()) { exit(1); }
  // CGEventTap
  CFMachPortRef eventTap;
  CGEventMask eventMask;
  CFRunLoopSourceRef runLoopSource;

  eventMask = (1 << kCGEventKeyDown) | (1 << kCGEventKeyUp);

  eventTap = CGEventTapCreate(kCGSessionEventTap, kCGHeadInsertEventTap,
                              kCGEventTapOptionDefault, eventMask,
                              myCGEventCallback, nullptr);

  if (!eventTap) {
    LOG_CRITICAL(
        "⚠️ "
        "创建事件监听失败！请开启辅助功能(Accessibility)权限或以sudo身份运行");
    exit(1);
  }

  // 在独立线程中启动鼠标事件监听（选中文字功能）
  start_mouse_event_listener();

  // WebSocket
  auto &ws_server = WebSocketServer::instance();
  std::thread ws_thread(&WebSocketServer::start_websocket_server, &ws_server);
  ws_thread.detach();
  runLoopSource =
      CFMachPortCreateRunLoopSource(kCFAllocatorDefault, eventTap, 0);
  CFRunLoopAddSource(CFRunLoopGetCurrent(), runLoopSource,
                     kCFRunLoopCommonModes);
  CFRelease(runLoopSource);
  CGEventTapEnable(eventTap, true);

  LOG_INFO("业务启动完成，进入事件循环");

  // 运行循环
  CFRunLoopRun();

  LOG_INFO("业务退出，开始清理资源");
  CGEventTapEnable(eventTap, false);
  CFRelease(eventTap);
}

int main(int argc, char *argv[]) {
  if (argc == 1) {
    run_main_business();
    exit(EXIT_SUCCESS);
  } else if (strcmp(argv[1], "--daemon") == 0) {
    // 1. 先变成守护进程
    init_daemon_early();

    // 2. 再启动所有业务（线程/CoreFoundation/CGEventTap）
    run_main_business();
    unlink(PID_FILE);
    if (g_log_fd >= 0) close(g_log_fd);
    LOG_INFO("守护进程正常退出");
    exit(EXIT_SUCCESS);
    return 0;
  }
}