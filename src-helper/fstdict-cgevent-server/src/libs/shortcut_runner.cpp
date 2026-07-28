#include "shortcut_runner.h"
#include "selection_monitor.h"
#include "../CGEventHandler.h"
#include "recursive-map.h"
#include "key_map.h"
#include "logger.h"

#include <cstdlib> // 用于 system() 执行shell命令
#include <string>
#include <iostream>
#include <algorithm>
#include <memory>
#include <future>

using namespace std;
using dblisp::recursive_map;

#define FLAGS_MODIFIER_COMMAND (flags & MODIFIER_COMMAND)
#define FLAGS_MODIFIER_OPTION (flags & MODIFIER_OPTION)
#define FLAGS_MODIFIER_CTRL (flags & MODIFIER_CTRL)
#define FLAGS_MODIFIER_SHIFT (flags & MODIFIER_SHIFT)

// 执行自定义Shell命令/脚本
void ShortcutRunner::execute_shell_command(const string &cmd)
{
    LOG_INFO("[执行命令] {}", cmd);
    // 用system()执行，也可以用popen/pclose获取输出（按需扩展）
    int ret = system(cmd.c_str());
    if (ret != 0)
    {
        LOG_WARN("[命令执行失败] 退出码: {}", ret);
    }
}

// 异步执行命令（不阻塞）
void ShortcutRunner::execute_shell_command_async(const string &cmd)
{
    LOG_INFO("[异步执行命令] {}", cmd);

    // std::async 启动独立线程执行system，立即返回
    auto future = async(launch::async, [cmd]()
                        {
        int ret = system(cmd.c_str());
        if (ret != 0)
        {
            LOG_WARN("[命令执行失败] 退出码: {}", ret);
        } });

    // 关键：不要调用 future.get() / wait()，否则会重新阻塞！
    // 线程会自动在后台执行完成
}

void ShortcutRunner::execute_shell_command_fork(const string &cmd)
{
    LOG_INFO("[fork后台执行] {}", cmd);

    pid_t pid = fork();
    if (pid == 0)
    {
        // ===================== 【关键】子进程内部关闭FD =====================
        // 只关闭【子进程自己】的文件描述符，主进程不受任何影响！
        // 0 1 2 是标准输入输出，保留；从3开始关闭所有socket、端口
        for (int fd = 3; fd < 256; fd++)
        {
            close(fd);
        }
        // ====================================================================

        // 让子进程彻底脱离父进程，变成独立后台进程
        setsid();

        // 执行你的 shell 命令
        execl("/bin/sh", "sh", "-c", cmd.c_str(), nullptr);
        _exit(1);
    }
    else if (pid > 0)
    {
        // 父进程：直接返回，继续运行 WebSocket 服务，完全不受影响
        return;
    }
    else
    {
        LOG_ERROR("[fork创建进程失败]");
    }
}

void ShortcutRunner::__selection_monitor_callback(const std::string &selected_text)
{
    if (!selection_monitor_callback_cmd.empty())
    {
        execute_shell_command_fork(selection_monitor_callback_cmd + " " + selected_text);
    }
}

// 切换选择监控状态
void ShortcutRunner::__toggle_selection_monitor()
{
    if (!selection_monitor_enabled)
    {
        if (start_selection_monitor([this](const string &text)
                                    { this->__selection_monitor_callback(text); }))
        {
            selection_monitor_enabled = true;
            const std::string cmd = "terminal-notifier -title \"Selection Monitor\" -message \"Selection Monitor Started\"";
            execute_shell_command_fork(cmd);
        }
        else
        {
            const std::string cmd = "terminal-notifier -title \"Selection Monitor\" -message \"Selection Monitor Failed to Start\"";
            execute_shell_command_fork(cmd);
        }
    }
    else
    {
        stop_selection_monitor();
        selection_monitor_enabled = false;
        const std::string cmd = "terminal-notifier -title \"Selection Monitor\" -message \"Selection Monitor Stopped\"";
        execute_shell_command_fork(cmd);
    }
}

void ShortcutRunner::__read_shortcut_config()
{
    // 从config递归映射中读取快捷键配置，构建内部数据结构（按需实现）
    // 例如，可以将配置解析成一个map<组合标识, 命令>的形式，便于快速匹配和执行
    for (auto &item : config)
    {
        unsigned char index = 0;
        if (item.key().toString() == "settings")
        {
            continue; // 跳过settings项
        }
        const auto &keys_ = item["keys"].valueVector();
        if (keys_.empty())
        {
            continue; // 跳过没有配置按键的项
        }
        auto keys = std::vector<std::string>(keys_.begin(), keys_.end());
        if (std::find(keys.begin(), keys.end(), "command") != keys.end())
        {
            index |= 1 << 0; // Command
        }
        if (std::find(keys.begin(), keys.end(), "option") != keys.end())
        {
            index |= 1 << 1; // Option
        }
        if (std::find(keys.begin(), keys.end(), "control") != keys.end())
        {
            index |= 1 << 2; // Control
        }
        if (std::find(keys.begin(), keys.end(), "shift") != keys.end())
        {
            index |= 1 << 3; // Shift
        }
        const std::string &last_key = keys[keys.size() - 1];
        auto keycode = findKeyCode(last_key);
        if (keycode)
        {
            if (shortcuts_map[index] == nullptr)
            {
                shortcuts_map[index] = std::make_unique<std::unordered_map<std::string, dblisp::recursive_map>>();
            }
            (*shortcuts_map[index])[std::to_string(keycode.value())] = item;
        }
    }

    for (size_t i = 0; i < shortcuts_map.size(); i++)
    {
        if (shortcuts_map[i] != nullptr)
        {
            LOG_DEBUG("index: {}", i);
            for (auto &p : *shortcuts_map[i])
            {
                LOG_DEBUG("  {}", p.first);
                LOG_INFO("{}", p.second.formatLisp());
            }
        }
    }
}

// 检查快捷键组合（核心逻辑）
bool ShortcutRunner::check_shortcut(CGEventRef event)
{
    // 1. 获取当前按键的修饰键状态（Command/Option/Control/Shift）
    CGEventFlags flags = CGEventGetFlags(event);

    // 2. 获取普通按键的KeyCode
    CGKeyCode keycode = (CGKeyCode)CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode);

    unsigned char index = 0;

    if (FLAGS_MODIFIER_COMMAND)
    {
        index |= 1 << 0; // Command
    }
    if (FLAGS_MODIFIER_OPTION)
    {
        index |= 1 << 1; // Option
    }
    if (FLAGS_MODIFIER_CTRL)
    {
        index |= 1 << 2; // Control
    }
    if (FLAGS_MODIFIER_SHIFT)
    {
        index |= 1 << 3; // Shift
    }
    if (shortcuts_map[index] == nullptr)
    {
        return false;
    }
    auto it = shortcuts_map[index]->find(std::to_string(keycode));
    if (it == shortcuts_map[index]->end())
    {
        return false;
    }
    if (last_time.find(keycode) != last_time.end())
    {

        Microseconds_ microseconds = std::chrono::duration_cast<std::chrono::microseconds>(
                                         std::chrono::high_resolution_clock::now() - last_time[keycode])
                                         .count();
        if (microseconds < double_typing_interval)
        {

            LOG_INFO("拦截快捷键: {}", it->second.formatLisp());
            LOG_INFO("间隔: {} 微秒", microseconds); // 输出被拦截的按键和间隔时间，便于调试
            return false;                            // 拦截该事件
        }
    }
    auto &item = it->second;
    const auto &cmd = item["command"].value().asString();
    const std::string shortcut_name = item.key().toString();

    if (shortcut_name == "toggle shortcut")
    {
        shortcut_enabled = !shortcut_enabled;
        const std::string not_cmd = "terminal-notifier -title \"Keyboard:Shortcut\" -message \"" + (shortcut_enabled ? string("Enabled") : string("Disabled")) + "\"";
        execute_shell_command_fork(not_cmd);
        return true; // 拦截快捷键，不传递事件
    }
    if (!shortcut_enabled)
    {
        return false;
    }

    if (shortcut_name == "reload config")
    {
        cgEventHandlerRef.load_config();
        execute_shell_command_fork(cmd);
    }
    // else if (shortcut_name == "toggle selection monitor")
    // {
    //     __toggle_selection_monitor();
    // }
    // else if (shortcut_name == "toggle selection monitor callback")
    // {
    //     __toggle_selection_monitor_callback(cmd);
    // }
    // else if (shortcut_name == "handle selection")
    // { // bug
    //     handleSelection("**", "**");
    // }
    else if (shortcut_name == "exit")
    {
        LOG_INFO("停止事件循环...");
        const std::string not_cmd = "terminal-notifier -title \"Keyboard\" -message \"Exit!\"";
        execute_shell_command_fork(not_cmd);
        // 停止主程序的 RunLoop，安全退出
        CFRunLoopStop(CFRunLoopGetCurrent());
        return true;
    }
    else
    {
        // 其他快捷键，直接执行命令
        execute_shell_command_fork(cmd);
    }
    return true; // 拦截快捷键，不传递事件
}

void ShortcutRunner::__toggle_selection_monitor_callback(const std::string &cmd)
{
    selection_monitor_callback_cmd = cmd;
    if (!selection_monitor_callback_enabled)
    {
        set_selection_monitor_callback([this](const std::string &text)
                                       { __selection_monitor_callback(text); });
        selection_monitor_callback_enabled = true;
        const std::string not_cmd = "terminal-notifier -title \"Selection Monitor Callback\" -message \"Started!\"";
        execute_shell_command_fork(not_cmd);
    }
    else
    {
        set_selection_monitor_callback(nullptr);
        selection_monitor_callback_enabled = false;
        const std::string not_cmd = "terminal-notifier -title \"Selection Monitor Callback\" -message \"Stopped!\"";
        execute_shell_command_fork(not_cmd);
    }
}
