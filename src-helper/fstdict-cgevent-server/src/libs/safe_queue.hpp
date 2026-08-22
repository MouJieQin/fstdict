#pragma once
#include <condition_variable>
#include <mutex>
#include <optional>
#include <queue>

/// Thread-safe FIFO queue for producer-consumer patterns
/// Uses condition variable for zero-CPU blocking wait
template <typename T> class SafeQueue {
public:
  /// Push an element into the queue (non-blocking)
  void push(T data) {
    std::lock_guard<std::mutex> lock(m_mutex);
    m_queue.push(std::move(data));
    m_cv.notify_one();
  }

  /// Block until an element is available, then pop and return it
  T waitAndPop() {
    std::unique_lock<std::mutex> lock(m_mutex);
    m_cv.wait(lock, [this]() { return !m_queue.empty(); });

    T data = std::move(m_queue.front());
    m_queue.pop();
    return data;
  }

  /// Try to pop an element without blocking; returns nullopt if empty
  std::optional<T> tryPop() {
    std::lock_guard<std::mutex> lock(m_mutex);
    if (m_queue.empty()) return std::nullopt;

    T data = std::move(m_queue.front());
    m_queue.pop();
    return data;
  }

  /// Check if queue is empty
  bool empty() const {
    std::lock_guard<std::mutex> lock(m_mutex);
    return m_queue.empty();
  }

private:
  std::queue<T> m_queue;
  mutable std::mutex m_mutex;
  std::condition_variable m_cv;
};
