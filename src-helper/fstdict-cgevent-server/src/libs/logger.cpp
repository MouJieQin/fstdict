#include "logger.h"

Logger &Logger::instance() {
  static Logger logger;
  return logger;
}

std::shared_ptr<spdlog::logger> Logger::getLogger() const { return m_logger; }

void Logger::setLevel(spdlog::level::level_enum level) {
  m_logger->set_level(level);
}

Logger::Logger() {
  try {
    // Create dual sinks: colored console + rotating file
    auto consoleSink = std::make_shared<spdlog::sinks::stdout_color_sink_mt>();
    auto logPath = (getAppLogDir() / LOG_FILE_NAME).u8string();
    auto logPathStr = std::string(
        reinterpret_cast<const char *>(logPath.data()), logPath.size());
    auto fileSink = std::make_shared<spdlog::sinks::rotating_file_sink_mt>(
        logPathStr, LOG_MAX_SIZE, LOG_MAX_FILES);

    // Apply unified format to both sinks
    consoleSink->set_pattern(LOG_PATTERN);
    fileSink->set_pattern(LOG_PATTERN);

    // Create multi-sink logger
    m_logger = std::make_shared<spdlog::logger>(
        "multi_sink", spdlog::sinks_init_list{consoleSink, fileSink});

    m_logger->set_level(spdlog::level::info);
    m_logger->flush_on(spdlog::level::warn);
    spdlog::flush_every(std::chrono::seconds(3));

    // Register as global default logger
    spdlog::set_default_logger(m_logger);
  } catch (const spdlog::spdlog_ex &ex) {
    printf("Logger initialization failed: %s\n", ex.what());
  }
}
