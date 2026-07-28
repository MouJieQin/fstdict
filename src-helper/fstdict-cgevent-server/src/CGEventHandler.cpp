#include "CGEventHandler.h"
#include "libs/logger.h"
#include "libs/shortcut_runner.h"
#include <chrono>
#include <iostream>
#include <memory>
#include <string>
#include <unordered_map>
#include <fstream>

using namespace std;
using json = nlohmann::json;
unordered_map<CGKeyCode, Time> last_time;
CGEventHandler cgEventHandler; // 全局实例，确保在事件回调中可用

inline std::filesystem::path
CGEventHandler::get_config_path(const std::string &app_name) {
  std::filesystem::path config_path;

  // macOS: ~/Library/Application Support/app_name
  const char *home = std::getenv("HOME");
  config_path =
      (home ? std::filesystem::path(reinterpret_cast<const char8_t *>(home))
            : std::filesystem::path("~")) /
      "Library" / "Application Support" / app_name / "Storage" / "config" /
      "cgevent_config.json";
  return config_path;
}

void CGEventHandler::load_config() {
  std::filesystem::path config_path = get_config_path("FstDict");
  std::ifstream ifs(config_path);
  if (!ifs) {
    LOG_CRITICAL("Failed to open file {} for reading.", config_path.string());
    exit(1);
  }
  json config;
  try {
    ifs >> config;
  } catch (const json::exception &e) {
    LOG_CRITICAL("Config JSON file {} format error: {}", config_path.string(),
                 e.what());
    exit(1);
  } catch (const std::exception &e) {
    LOG_CRITICAL("Config JSON file {} read error: {}", config_path.string(),
                 e.what());
    exit(1);
  }
  config_ = std::move(config);

  shortcutRunnerPtr_ = make_shared<ShortcutRunner>(config_["shortcuts"],
                                                   cgEventHandler, last_time);
}

// 检查快捷键组合
bool CGEventHandler::check_shortcut(CGEventRef event) {
  return shortcutRunnerPtr_->check_shortcut(event);
}

// 获取当前时间（防连击用）
Time time_now() { return chrono::high_resolution_clock::now(); }

// 事件回调函数
CGEventRef myCGEventCallback(CGEventTapProxy proxy, CGEventType type,
                             CGEventRef event, void *refcon) {
  // 只处理按键按下事件（KeyDown）
  if (type != kCGEventKeyDown) {
    // 按键抬起时记录时间（防连击用）
    if (type == kCGEventKeyUp) {
      CGKeyCode keycode = (CGKeyCode)CGEventGetIntegerValueField(
          event, kCGKeyboardEventKeycode);
      last_time[keycode] = time_now();
    }
    return event;
  }

  // 第一步：检查是否触发自定义快捷键
  if (cgEventHandler.check_shortcut(event)) {
    return NULL; // 拦截快捷键事件，不传递给系统
  }
  // 正常传递事件
  return event;
}
