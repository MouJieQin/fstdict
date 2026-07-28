#ifndef __KEY_MAP_H__
#define __KEY_MAP_H__

#include <ApplicationServices/ApplicationServices.h>
#include <algorithm>
#include <array>
#include <optional>
#include <string>

// ====================== 1. 编译期键值表（核心数据） ======================
consteval auto getKeyMappings() {
  using Pair = std::pair<std::string_view, CGKeyCode>;

  return std::array{
      Pair{"a", 0},          Pair{"s", 1},      Pair{"d", 2},
      Pair{"f", 3},          Pair{"h", 4},      Pair{"g", 5},
      Pair{"z", 6},          Pair{"x", 7},      Pair{"c", 8},
      Pair{"v", 9},          Pair{"b", 11},     Pair{"q", 12},
      Pair{"w", 13},         Pair{"e", 14},     Pair{"r", 15},
      Pair{"y", 16},         Pair{"t", 17},     Pair{"1", 18},
      Pair{"2", 19},         Pair{"3", 20},     Pair{"4", 21},
      Pair{"6", 22},         Pair{"5", 23},     Pair{"=", 24},
      Pair{"9", 25},         Pair{"7", 26},     Pair{"-", 27},
      Pair{"8", 28},         Pair{"0", 29},     Pair{"]", 30},
      Pair{"o", 31},         Pair{"u", 32},     Pair{"[", 33},
      Pair{"i", 34},         Pair{"p", 35},     Pair{"l", 37},
      Pair{"j", 38},         Pair{"'", 39},     Pair{"k", 40},
      Pair{";", 41},         Pair{"\\", 42},    Pair{",", 43},
      Pair{"/", 44},         Pair{"n", 45},     Pair{"m", 46},
      Pair{".", 47},         Pair{"`", 50},     Pair{"return", 36},
      Pair{"tab", 48},       Pair{"space", 49}, Pair{"delete", 51},
      Pair{"backspace", 51}, Pair{"esc", 53},   Pair{"command", 55},
      Pair{"cmd", 55},       Pair{"shift", 56}, Pair{"ctrl", 59},
      Pair{"control", 59},   Pair{"opt", 58},   Pair{"option", 58}};
}

// ====================== 2. 字符串 → 键码（编译期查找） ======================
constexpr std::optional<CGKeyCode> findKeyCode(std::string_view key) {
  constexpr auto map = getKeyMappings();
  for (const auto &[k, v] : map) {
    if (k == key) return v;
  }
  return std::nullopt;
}
// ====================== 3. 键码 → 字符串（编译期查找） ======================
constexpr std::optional<std::string_view> findKeyName(CGKeyCode code) {
  constexpr auto map = getKeyMappings();
  for (const auto &[k, v] : map) {
    if (v == code) return k;
  }
  return std::nullopt;
}

// ====================== 对外兼容接口 ======================
inline CGKeyCode getKeyCode(const std::string &key) {
  return findKeyCode(key).value_or(0xff);
}

inline std::string getKeyName(CGKeyCode code) {
  return std::string(findKeyName(code).value_or("unknown"));
}

#endif // __KEY_MAP_H__