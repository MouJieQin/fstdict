#include "selection_monitor.h"
#include "accessibility_manager.h"
#include "clipboard_manager.h"
#include "logger.h"
#include "websocket_server.h"
#import <Cocoa/Cocoa.h>

#include <atomic>
#include <chrono>
#include <mutex>
#include <optional>
#include <thread>

using namespace std::chrono_literals;

// Configuration constants
constexpr uint64_t DOUBLE_CLICK_INTERVAL_NS = 350'000'000; // 350 ms
constexpr uint64_t LONG_PRESS_DURATION_NS = 500'000'000;   // 500 ms
constexpr auto TRIGGER_DELAY = 1ms;    // Delay before reading selection
constexpr auto COPY_WAIT_TIME = 200ms; // Wait for clipboard to update

// Internal state
namespace {
std::mutex g_stateMutex;
std::string g_lastSelectedText;
std::string g_originalClipboard;
std::atomic<bool> g_isProcessing{false};

uint64_t g_lastMouseDownTimestamp = 0;
uint64_t g_lastMouseUpTimestamp = 0;
bool g_isDoubleClick = false;
CGPoint g_mouseDownLocation{};
CGPoint g_mouseUpLocation{};

CFMachPortRef g_eventTap = nullptr;
CFRunLoopSourceRef g_runLoopSource = nullptr;

auto &g_wsServer = WebSocketServer::instance();
} // namespace

/// Try to get selected text via Accessibility API (preferred, non-intrusive)
static std::optional<std::string> getSelectedTextViaAX() {
  NSRunningApplication *frontApp =
      [[NSWorkspace sharedWorkspace] frontmostApplication];
  if (!frontApp) {
    LOG_WARN("Cannot retrieve frontmost application");
    return std::nullopt;
  }

  pid_t pid = [frontApp processIdentifier];
  AXUIElementRef appRef = AXUIElementCreateApplication(pid);
  AXUIElementRef focusedElement = nullptr;

  AXError err = AXUIElementCopyAttributeValue(
      appRef, kAXFocusedUIElementAttribute, (CFTypeRef *)&focusedElement);

  std::optional<std::string> result = std::nullopt;

  if (err == kAXErrorSuccess && focusedElement) {
    CFTypeRef selectedText = nullptr;
    err = AXUIElementCopyAttributeValue(
        focusedElement, kAXSelectedTextAttribute, &selectedText);

    if (err == kAXErrorSuccess && selectedText) {
      NSString *text = (__bridge NSString *)selectedText;
      std::string utf8Text([text UTF8String]);

      if (!utf8Text.empty()) {
        result = utf8Text;
        LOG_INFO("Got selected text via AX from [{}]: {}",
                 [[frontApp localizedName] UTF8String], utf8Text);
      } else {
        LOG_INFO("Focused element found in [{}], but no text selected",
                 [[frontApp localizedName] UTF8String]);
      }
      CFRelease(selectedText);
    } else {
      LOG_INFO("Focused element found in [{}], but no selection attribute",
               [[frontApp localizedName] UTF8String]);
    }
    CFRelease(focusedElement);
  } else {
    LOG_WARN("Failed to get focused element from [{}], error code: {}",
             [[frontApp localizedName] UTF8String], static_cast<int>(err));
  }

  CFRelease(appRef);
  return result;
}

/// Fallback: get selected text by simulating Cmd+C
static std::string getSelectedTextViaSimulatedCopy() {
  std::this_thread::sleep_for(TRIGGER_DELAY);

  g_originalClipboard = getClipboardText();
  simulateCopyShortcut();
  std::this_thread::sleep_for(COPY_WAIT_TIME);

  std::string selected = getClipboardText();
  std::this_thread::sleep_for(COPY_WAIT_TIME);

  // Restore original clipboard content
  setClipboardText(g_originalClipboard);
  return selected;
}

/// Process text selection event and broadcast via WebSocket
static void processSelectionEvent() {
  if (g_isProcessing.exchange(true)) {
    return; // Skip if already processing
  }

  auto axResult = getSelectedTextViaAX();
  std::string selectedText;

  if (axResult.has_value()) {
    selectedText = std::move(axResult.value());
  } else {
    selectedText = getSelectedTextViaSimulatedCopy();
  }

  if (!selectedText.empty() && selectedText != g_originalClipboard) {
    LOG_INFO("Text selection captured: {}", selectedText);

    nlohmann::json eventData;
    eventData["type"] = "CGEvent";
    eventData["data"]["type"] =
        EventTypeUtil::toString(EventType::kHandlerTextSelection);
    eventData["data"]["text_selected"] = selectedText;
    eventData["data"]["mouseLocation_down"]["x"] = g_mouseDownLocation.x;
    eventData["data"]["mouseLocation_down"]["y"] = g_mouseDownLocation.y;
    eventData["data"]["mouseLocation_up"]["x"] = g_mouseUpLocation.x;
    eventData["data"]["mouseLocation_up"]["y"] = g_mouseUpLocation.y;

    g_wsServer.pushEvent(eventData);
  }

  g_isProcessing = false;
}

/// Core CGEvent tap callback for mouse events
CGEventRef mouseEventCallback(CGEventTapProxy _, CGEventType type,
                              CGEventRef event, void *) {
  uint64_t timestamp = CGEventGetTimestamp(event);

  if (type == kCGEventLeftMouseDown) {
    g_mouseDownLocation = CGEventGetLocation(event);
    uint64_t timeSinceLastUp = timestamp - g_lastMouseUpTimestamp;
    g_isDoubleClick = (timeSinceLastUp < DOUBLE_CLICK_INTERVAL_NS);
    g_lastMouseDownTimestamp = timestamp;

    // Broadcast mouse down event if subscribed
    if (g_wsServer.isEventSubscribed(EventType::kCGEventLeftMouseDown)) {
      nlohmann::json eventData;
      eventData["type"] = "CGEvent";
      eventData["data"]["type"] =
          EventTypeUtil::toString(EventType::kCGEventLeftMouseDown);
      eventData["data"]["x"] = g_mouseDownLocation.x;
      eventData["data"]["y"] = g_mouseDownLocation.y;
      eventData["data"]["timestamp"] = timestamp;
      g_wsServer.pushEvent(eventData);
    }
  } else if (type == kCGEventLeftMouseUp) {
    g_mouseUpLocation = CGEventGetLocation(event);
    g_lastMouseUpTimestamp = timestamp;

    bool shouldTrigger =
        g_isDoubleClick ||
        ((timestamp - g_lastMouseDownTimestamp) > LONG_PRESS_DURATION_NS);

    if (shouldTrigger &&
        g_wsServer.isEventSubscribed(EventType::kHandlerTextSelection)) {
      std::thread(processSelectionEvent).detach();
    }
  }

  return event;
}

bool startMouseEventListener() {
  g_eventTap = CGEventTapCreate(kCGSessionEventTap, kCGHeadInsertEventTap,
                                kCGEventTapOptionListenOnly,
                                CGEventMaskBit(kCGEventLeftMouseDown) |
                                    CGEventMaskBit(kCGEventLeftMouseUp),
                                mouseEventCallback, nullptr);

  if (!g_eventTap) {
    LOG_CRITICAL("Failed to create event tap. "
                 "Please verify Accessibility permissions.");
    return false;
  }

  g_runLoopSource = CFMachPortCreateRunLoopSource(nullptr, g_eventTap, 0);
  CFRunLoopAddSource(CFRunLoopGetCurrent(), g_runLoopSource,
                     kCFRunLoopCommonModes);
  CGEventTapEnable(g_eventTap, true);

  LOG_INFO("Mouse event listener started, entering run loop");
  CFRunLoopRun();

  // Cleanup after run loop exits
  LOG_INFO("Event loop stopped, cleaning up resources");

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
