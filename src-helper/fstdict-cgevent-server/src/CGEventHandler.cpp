#include "CGEventHandler.h"
#include <iostream>
#include <chrono>
#include <unordered_map>
#include <string>
#include <memory>
#include "libs/shortcut_runner.h"
#include "libs/dblisp-parser.h"
#include "libs/logger.h"

using namespace std;
using dblisp::DbLispParser;
using dblisp::recursive_map;

unordered_map<CGKeyCode, Time> last_time;
CGEventHandler cgEventHandler; // 全局实例，确保在事件回调中可用

void CGEventHandler::load_config()
{
    DbLispParser parser;
    // 加载配置（示例路径和文件名，请根据实际情况修改）
    if (!parser.lispToRecMap("config.scm", config))
    {
        LOG_CRITICAL("加载快捷键配置失败！请检查配置文件格式和路径。");
        exit(1);
    }

    shortcutRunnerPtr = make_shared<ShortcutRunner>(config["shortcuts"], cgEventHandler, last_time);
}

// 检查快捷键组合
bool CGEventHandler::check_shortcut(CGEventRef event)
{
    return shortcutRunnerPtr->check_shortcut(event);
}

// 获取当前时间（防连击用）
Time time_now()
{
    return chrono::high_resolution_clock::now();
}

// 事件回调函数
CGEventRef myCGEventCallback(CGEventTapProxy proxy, CGEventType type, CGEventRef event, void *refcon)
{
    // 只处理按键按下事件（KeyDown）
    if (type != kCGEventKeyDown)
    {
        // 按键抬起时记录时间（防连击用）
        if (type == kCGEventKeyUp)
        {
            CGKeyCode keycode = (CGKeyCode)CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode);
            last_time[keycode] = time_now();
        }
        return event;
    }

    // 第一步：检查是否触发自定义快捷键
    if (cgEventHandler.check_shortcut(event))
    {
        return NULL; // 拦截快捷键事件，不传递给系统
    }
    // 正常传递事件
    return event;
}
