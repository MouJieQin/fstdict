#ifndef SELECTION_MONITOR_H
#define SELECTION_MONITOR_H

#include <functional>
#include <string>

// 对外回调类型：选中文字时触发
using SelectionCallback = std::function<void(const std::string &selected_text)>;

bool ensureAccessibility();

// 初始化并启动监听
bool start_mouse_event_listener();
bool start_selection_monitor(SelectionCallback callback = nullptr);

// 设置选中文字时触发回调
void set_selection_monitor_callback(SelectionCallback callback = nullptr);

// 停止监听
void stop_selection_monitor();

void handleSelection(const std::string &prefix, const std::string &suffix);

// 获取最后一次选中的文字
std::string get_last_selected_text();

#endif
