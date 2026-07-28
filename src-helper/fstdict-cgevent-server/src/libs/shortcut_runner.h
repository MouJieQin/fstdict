#ifndef __SHORTCUT_RUNNER_H__
#define __SHORTCUT_RUNNER_H__

#include <ApplicationServices/ApplicationServices.h>
#include <memory>

#include <nlohmann/json.hpp>

// 修饰键状态常量（CoreGraphics 定义）
#define MODIFIER_SHIFT (1 << 17)
#define MODIFIER_CTRL (1 << 18)
#define MODIFIER_OPTION (1 << 19)
#define MODIFIER_COMMAND (1 << 20)

typedef std::chrono::time_point<std::chrono::high_resolution_clock> Time;
typedef long long Microseconds_;

class CGEventHandler;

class ShortcutRunner {
public:
  ShortcutRunner(const nlohmann::json &shortcut_config,
                 CGEventHandler &cgEventHandlerRef,
                 std::unordered_map<CGKeyCode, Time> &last_time)
      : config(shortcut_config), double_typing_interval(500000),
        shortcuts_map(16, nullptr), shortcut_enabled(true),
        selection_monitor_enabled(false),
        selection_monitor_callback_enabled(false),
        selection_monitor_callback_cmd(""),
        cgEventHandlerRef(cgEventHandlerRef), last_time(last_time) {
    __read_shortcut_config();
  }
  ~ShortcutRunner() = default;

  // 检查快捷键组合
  bool check_shortcut(CGEventRef event);

private:
  void __read_shortcut_config();

  void __toggle_selection_monitor();

  void __toggle_selection_monitor_callback(const std::string &cmd);

  void __selection_monitor_callback(const std::string &selected_text);

  // 执行自定义Shell命令/脚本
  void execute_shell_command(const std::string &cmd);

  void execute_shell_command_async(const std::string &cmd);

  void execute_shell_command_fork(const std::string &cmd);

private:
  nlohmann::json config;
  long long double_typing_interval; // 连击时间间隔，单位微秒
  std::vector<std::shared_ptr<std::unordered_map<std::string, nlohmann::json>>>
      shortcuts_map; // 内部快捷键映射（按需设计数据结构）
  bool shortcut_enabled;
  bool selection_monitor_enabled;
  bool selection_monitor_callback_enabled;
  std::string selection_monitor_callback_cmd;
  CGEventHandler &cgEventHandlerRef;
  std::unordered_map<CGKeyCode, Time> &last_time;
};

#endif
