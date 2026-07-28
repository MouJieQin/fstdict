#ifndef __CG_EVENT_HANDLER_H__
#define __CG_EVENT_HANDLER_H__
#include "libs/recursive-map.h"
#include <ApplicationServices/ApplicationServices.h>

CGEventRef myCGEventCallback(CGEventTapProxy proxy, CGEventType type, CGEventRef event, void *refcon);

class ShortcutRunner;

class CGEventHandler
{
public:
    explicit CGEventHandler() : config("config")
    {
        load_config();
    }
    ~CGEventHandler() = default;

    void load_config();

    // 检查快捷键组合
    bool check_shortcut(CGEventRef event);

    // 检查是否触发连击
    bool check_double_typing(CGEventRef event);

private:
    dblisp::recursive_map config;
    std::shared_ptr<ShortcutRunner> shortcutRunnerPtr;
};
#endif