#ifndef __CG_EVENT_HANDLER_H__
#define __CG_EVENT_HANDLER_H__
#include <ApplicationServices/ApplicationServices.h>
#include <filesystem>
#include <nlohmann/json.hpp>

CGEventRef myCGEventCallback(CGEventTapProxy proxy, CGEventType type,
                             CGEventRef event, void *refcon);

class ShortcutRunner;

class CGEventHandler {
public:
  explicit CGEventHandler() : shortcutRunnerPtr_(nullptr) { load_config(); }
  ~CGEventHandler() = default;

  void load_config();

  // 检查快捷键组合
  bool check_shortcut(CGEventRef event);

private:
  inline std::filesystem::path
  get_config_path(const std::string &app_name = "FstDict");

  private:
    nlohmann::json config_;
    std::shared_ptr<ShortcutRunner> shortcutRunnerPtr_;
  };
#endif