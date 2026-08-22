#pragma once

#include <cstdlib>
#include <filesystem>
#include <memory>
#include <spdlog/sinks/rotating_file_sink.h>
#include <spdlog/sinks/stdout_color_sinks.h>
#include <spdlog/spdlog.h>

/// Check if standard output is attached to a terminal
#ifdef _WIN32
#include <windows.h>
inline bool isTerminal() {
  DWORD mode;
  return GetConsoleMode(GetStdHandle(STD_OUTPUT_HANDLE), &mode) != 0;
}
#else
#include <unistd.h>
inline bool isTerminal() { return isatty(STDOUT_FILENO) == 1; }
#endif

/// Get platform-standard application log directory
/// Falls back to ./logs if system paths are unavailable
inline std::filesystem::path
getAppLogDir(const std::string &appName = "FstDict") {
  std::filesystem::path logDir;

#ifdef _WIN32
  const char *localAppdata = std::getenv("LOCALAPPDATA");
  if (localAppdata) {
    logDir =
        std::filesystem::path(reinterpret_cast<const char8_t *>(localAppdata)) /
        appName / "Logs";
  }
#elif __APPLE__
  const char *home = std::getenv("HOME");
  if (home) {
    logDir = std::filesystem::path(reinterpret_cast<const char8_t *>(home)) /
             "Library" / "Logs" / appName;
  }
#elif __linux__
  const char *home = std::getenv("HOME");
  if (home) {
    logDir = std::filesystem::path(reinterpret_cast<const char8_t *>(home)) /
             ".local" / "share" / appName / "logs";
  }
#endif

  if (logDir.empty()) { logDir = std::filesystem::current_path() / "logs"; }

  try {
    std::filesystem::create_directories(logDir);
  } catch (const std::filesystem::filesystem_error &e) {
    printf("Failed to create log directory: %s\n", e.what());
  }

  return logDir;
}

// Log configuration constants
constexpr const char *LOG_FILE_NAME = "fstdict-cgevent-server.log";
constexpr size_t LOG_MAX_SIZE = 1024 * 1024 * 5; // 5 MB per file
constexpr size_t LOG_MAX_FILES = 1;              // Keep 1 rotated backup
constexpr const char *LOG_PATTERN =
    "[%Y-%m-%d %H:%M:%S] [%^%l%$] [thread %t] [%s:%#] %v";

/// Singleton logger with dual sinks (console + rotating file)
class Logger {
public:
  static Logger &instance();

  std::shared_ptr<spdlog::logger> getLogger() const;
  void setLevel(spdlog::level::level_enum level);

  Logger(const Logger &) = delete;
  Logger &operator=(const Logger &) = delete;

private:
  Logger();
  std::shared_ptr<spdlog::logger> m_logger;
};

// Convenience logging macros
#define LOG_TRACE(...)                                                         \
  SPDLOG_LOGGER_TRACE(Logger::instance().getLogger(), __VA_ARGS__)
#define LOG_DEBUG(...)                                                         \
  SPDLOG_LOGGER_DEBUG(Logger::instance().getLogger(), __VA_ARGS__)
#define LOG_INFO(...)                                                          \
  SPDLOG_LOGGER_INFO(Logger::instance().getLogger(), __VA_ARGS__)
#define LOG_WARN(...)                                                          \
  SPDLOG_LOGGER_WARN(Logger::instance().getLogger(), __VA_ARGS__)
#define LOG_ERROR(...)                                                         \
  SPDLOG_LOGGER_ERROR(Logger::instance().getLogger(), __VA_ARGS__)
#define LOG_CRITICAL(...)                                                      \
  SPDLOG_LOGGER_CRITICAL(Logger::instance().getLogger(), __VA_ARGS__)
