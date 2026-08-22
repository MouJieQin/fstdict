#ifndef SELECTION_MONITOR_H
#define SELECTION_MONITOR_H

#include <functional>
#include <string>

bool ensureAccessibility();

// 初始化并启动监听
bool start_mouse_event_listener();

// 获取最后一次选中的文字
std::string get_last_selected_text();

#endif
