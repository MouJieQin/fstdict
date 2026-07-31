#include <algorithm>
#include <array>
#include <iostream>
#include <optional>
#include <string>

// 1. 你的枚举（完全不变）
enum class EventType {
  kCGEventLeftMouseDown,
  handlerEventTextSelection,
  globalKeyboardShortCut
};

// ===================== C++20 极简枚举工具 =====================
consteval auto getEnumNames(EventType) {
  return std::array<std::pair<std::string_view, EventType>, 3>{
      {{"kCGEventLeftMouseDown", EventType::kCGEventLeftMouseDown},
       {"handlerEventTextSelection", EventType::handlerEventTextSelection},
       {"globalKeyboardShortCut", EventType::globalKeyboardShortCut},
      }};
}

struct EventTypeEnum {
  // 核心：判断字符串是否存在（你要的功能）
  static constexpr bool exists(std::string_view name) {
    for (auto [n, e] : getEnumNames(EventType{}))
      if (n == name) return true;
    return false;
  }

  // 字符串 → 枚举
  static constexpr std::optional<EventType> fromString(std::string_view name) {
    for (auto [n, e] : getEnumNames(EventType{}))
      if (n == name) return e;
    return std::nullopt;
  }

  // 枚举 → 字符串
  static constexpr std::string_view toString(EventType e) {
    for (auto [n, val] : getEnumNames(EventType{}))
      if (val == e) return n;
    return "Unknown";
  }
};