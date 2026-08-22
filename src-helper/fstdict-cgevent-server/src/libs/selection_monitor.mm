#include <chrono>
#include <iostream>
#include <mutex>
#include <optional>
#include <string>
#include <thread>

#include <Cocoa/Cocoa.h>

#include "logger.h"
#include "selection_monitor.h"
#include "websocket_server.h"

static std::mutex g_mutex;
static std::string g_last_selected;
static std::string g_old_text_in_clipboard;
static auto &g_websocket_server = WebSocketServer::instance();

using namespace std;
using namespace std::chrono;

// ===================== 可配置参数 =====================
#define DOUBLE_CLICK_INTERVAL 350000000 // 双击最大间隔(ns)
#define LONG_PRESS_DURATION 500000000   // 长按最小时长(ns)
#define DELAY_AFTER_TRIGGER 1           // 触发后等待多久获取文字(ms)
// ======================================================

static bool g_isProcessing = false;
static uint64_t g_lastMouseDown = 0;
static uint64_t g_lastMouseUp = 0;
static bool g_isDoubleClick = false;
static CGPoint g_mouseLocation_down;
static CGPoint g_mouseLocation_up;
static CFMachPortRef g_eventTap = nullptr;
static CFRunLoopSourceRef g_runLoopSource = nullptr;

// Check and Trigger Permissions
bool ensureAccessibility() {
  // Dictionary to tell macOS we want to prompt the user if permission is
  // missing
  NSDictionary *options = @{(id)kAXTrustedCheckOptionPrompt : @YES};
  bool trusted =
      AXIsProcessTrustedWithOptions((__bridge CFDictionaryRef)options);

  if (!trusted) {
    LOG_CRITICAL("⚠️ Permission denied. Please enable this app in System "
                 "Settings > Accessibility.");
  }
  return trusted;
}

// 剪贴板
string getClipboard() {
  NSPasteboard *pb = [NSPasteboard generalPasteboard];
  NSString *content = [pb stringForType:NSPasteboardTypeString];
  return content ? string([content UTF8String]) : "";
}

void setClipboard(const string &s) {
  NSPasteboard *pb = [NSPasteboard generalPasteboard];
  [pb clearContents];
  [pb setString:[NSString stringWithUTF8String:s.c_str()]
        forType:NSPasteboardTypeString];
}

void simulateCopy() {
  CGEventRef down = CGEventCreateKeyboardEvent(NULL, (CGKeyCode)8, true);
  CGEventSetFlags(down, kCGEventFlagMaskCommand);
  CGEventPost(kCGHIDEventTap, down);
  CFRelease(down);

  CGEventRef up = CGEventCreateKeyboardEvent(NULL, (CGKeyCode)8, false);
  CGEventSetFlags(up, kCGEventFlagMaskCommand);
  CGEventPost(kCGHIDEventTap, up);
  CFRelease(up);
}

optional<string> getSelectedText() {
  // 1. Get the frontmost application's PID
  NSRunningApplication *frontApp =
      [[NSWorkspace sharedWorkspace] frontmostApplication];
  if (!frontApp) {
    LOG_WARN(
        "❌ Could not find frontmost app while trying to get selected text.");
    return nullopt;
  }
  pid_t pid = [frontApp processIdentifier];

  // 2. Create an Accessibility object for that specific app
  AXUIElementRef appRef = AXUIElementCreateApplication(pid);
  AXUIElementRef focusedElement = NULL;

  // 3. Ask the APP for its focused element (more reliable than system-wide)
  AXError err = AXUIElementCopyAttributeValue(
      appRef, kAXFocusedUIElementAttribute, (CFTypeRef *)&focusedElement);

  optional<string> result = nullopt;
  if (err == kAXErrorSuccess && focusedElement) {
    CFTypeRef selectedText = NULL;
    err = AXUIElementCopyAttributeValue(
        focusedElement, kAXSelectedTextAttribute, &selectedText);

    if (err == kAXErrorSuccess && selectedText) {
      NSString *text = (__bridge NSString *)selectedText;
      result = string([text UTF8String]);
      if (result.value().empty()) {
        LOG_INFO("❓ [{}] Element found, but no text is selected.",
                 [[frontApp localizedName] UTF8String]);
        result = nullopt;
      } else {
        LOG_INFO("✅ Get selected text by AXUIElement from [{}]: {}",
                 [[frontApp localizedName] UTF8String], result.value());
      }
      CFRelease(selectedText);
    } else {
      LOG_INFO("❓ [{}] Element found , but no text is selected.",
               [[frontApp localizedName] UTF8String]);
    }
    CFRelease(focusedElement);
  } else {
    LOG_WARN("❌ Failed to get focused element from [{}] with error {}",
             [[frontApp localizedName] UTF8String], (int)err);
  }
  CFRelease(appRef);
  return result;
}

string getSelectedTextBySimulateCopy() {
  this_thread::sleep_for(milliseconds(DELAY_AFTER_TRIGGER));
  g_old_text_in_clipboard = getClipboard();
  simulateCopy();
  this_thread::sleep_for(milliseconds(300));
  string selected = getClipboard();
  this_thread::sleep_for(milliseconds(300));
  setClipboard(g_old_text_in_clipboard);
  return selected;
}

// 获取选中文字
void processSelection() {
  if (g_isProcessing) return;
  g_isProcessing = true;

  auto selected = getSelectedText();
  string selected_text = "";
  if (!selected) {
    selected_text = getSelectedTextBySimulateCopy();
  } else {
    selected_text = selected.value();
  }
  if (!selected_text.empty() && selected_text != g_old_text_in_clipboard) {
    LOG_INFO("✅ 捕获：{}", selected_text);
    json json_data;
    json_data["type"] = "CGEvent";
    json_data["data"]["type"] =
        EventTypeEnum::toString(EventType::handlerEventTextSelection);
    json_data["data"]["text_selected"] = selected_text;
    json_data["data"]["mouseLocation_down"]["x"] = g_mouseLocation_down.x;
    json_data["data"]["mouseLocation_down"]["y"] = g_mouseLocation_down.y;
    json_data["data"]["mouseLocation_up"]["x"] = g_mouseLocation_up.x;
    json_data["data"]["mouseLocation_up"]["y"] = g_mouseLocation_up.y;

    g_websocket_server.push_event_json(json_data);
  }

  g_isProcessing = false;
}

// 鼠标事件
CGEventRef mouseCallback(CGEventTapProxy proxy, CGEventType type,
                         CGEventRef event, void *refcon) {
  uint64_t now = CGEventGetTimestamp(event);

  if (type == kCGEventLeftMouseDown) {
    g_mouseLocation_down =
        CGEventGetLocation(event); // 核心函数：获取当前鼠标位置
    uint64_t diff = now - g_lastMouseUp;
    g_isDoubleClick = (diff < DOUBLE_CLICK_INTERVAL);
    g_lastMouseDown = now;
    // ====================== 新增：获取鼠标坐标 ======================
    if (g_websocket_server.is_need_to_listen(
            EventType::kCGEventLeftMouseDown)) {
      CGFloat x = g_mouseLocation_down.x; // 屏幕 X 坐标
      CGFloat y = g_mouseLocation_down.y; // 屏幕 Y 坐标

      json json_data;
      json_data["type"] = "CGEvent";
      json_data["data"]["type"] =
          EventTypeEnum::toString(EventType::kCGEventLeftMouseDown);
      json_data["data"]["x"] = x;
      json_data["data"]["y"] = y;
      json_data["data"]["timestamp"] = now;
      g_websocket_server.push_event_json(json_data);
    }
    // ==============================================================
  } else if (type == kCGEventLeftMouseUp) {
    g_mouseLocation_up =
        CGEventGetLocation(event); // 核心函数：获取当前鼠标位置
    g_lastMouseUp = now;
    bool trigger =
        g_isDoubleClick || ((now - g_lastMouseDown) > LONG_PRESS_DURATION);
    if (trigger) {
      if (g_websocket_server.is_need_to_listen(
              EventType::handlerEventTextSelection)) {
        thread(processSelection).detach();
      }
    }
  }
  return event;
}

// 启动监听
bool start_mouse_event_listener() {
  g_eventTap = CGEventTapCreate(kCGSessionEventTap, kCGHeadInsertEventTap,
                                kCGEventTapOptionListenOnly,
                                CGEventMaskBit(kCGEventLeftMouseDown) |
                                    CGEventMaskBit(kCGEventLeftMouseUp),
                                mouseCallback, NULL);

  if (!g_eventTap) {
    LOG_CRITICAL("❌ 请开启【辅助功能(Accessibility)】权限");
    return false;
  }

  g_runLoopSource = CFMachPortCreateRunLoopSource(NULL, g_eventTap, 0);
  CFRunLoopAddSource(CFRunLoopGetCurrent(), g_runLoopSource,
                     kCFRunLoopCommonModes);
  CGEventTapEnable(g_eventTap, true);
  LOG_INFO("业务启动完成，进入事件循环");
  // 运行循环
  CFRunLoopRun();
  LOG_INFO("业务退出，开始清理资源");
  if (g_eventTap) {
    CGEventTapEnable(g_eventTap, false);
    CFRelease(g_eventTap);
    g_eventTap = nullptr;
  }

  if (g_runLoopSource) {
    CFRunLoopRemoveSource(CFRunLoopGetCurrent(), g_runLoopSource,
                          kCFRunLoopCommonModes);
    CFRelease(g_runLoopSource);
    g_runLoopSource = nullptr;
  }
  return true;
}
