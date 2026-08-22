#pragma once
#include <array>
#include <optional>
#include <string_view>

/// Event types that clients can subscribe to
enum class EventType {
  kCGEventLeftMouseDown,
  kHandlerTextSelection,
};

/// Compile-time enum <-> string mapping
consteval auto getEventTypeMapping() {
  return std::array<std::pair<std::string_view, EventType>, 2>{{
      {"kCGEventLeftMouseDown", EventType::kCGEventLeftMouseDown},
      {"kHandlerTextSelection", EventType::kHandlerTextSelection},
  }};
}

struct EventTypeUtil {
  /// Check if an event name exists
  static constexpr bool exists(std::string_view name) {
    for (const auto &[name_str, event] : getEventTypeMapping()) {
      if (name_str == name) return true;
    }
    return false;
  }

  /// Convert string to EventType, returns nullopt if invalid
  static constexpr std::optional<EventType> fromString(std::string_view name) {
    for (const auto &[name_str, event] : getEventTypeMapping()) {
      if (name_str == name) return event;
    }
    return std::nullopt;
  }

  /// Convert EventType to string_view
  static constexpr std::string_view toString(EventType e) {
    for (const auto &[name_str, event] : getEventTypeMapping()) {
      if (event == e) return name_str;
    }
    return "Unknown";
  }
};
