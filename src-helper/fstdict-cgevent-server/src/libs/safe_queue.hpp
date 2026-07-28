#include <chrono>
#include <condition_variable>
#include <iostream>
#include <mutex>
#include <queue>
#include <thread>

// 线程安全队列（生产者-消费者）
template <typename T> class SafeQueue {
private:
  std::queue<T> queue;        // 数据队列
  std::mutex mtx;             // 互斥锁，保证线程安全
  std::condition_variable cv; // 条件变量：实现休眠/唤醒，无轮询

public:
  // 生产者：push 数据，不阻塞
  void push(T data) {
    std::lock_guard<std::mutex> lock(mtx);
    queue.push(std::move(data));
    cv.notify_one(); // 关键！唤醒消费者线程
  }

  // 消费者：等待并取出数据（无数据时自动休眠，0 CPU占用）
  T wait_and_pop() {
    std::unique_lock<std::mutex> lock(mtx);
    // 队列空 → 休眠，释放锁；被 notify 后 → 自动唤醒
    cv.wait(lock, [this]() { return !queue.empty(); });

    T data = std::move(queue.front());
    queue.pop();
    return data;
  }

  bool empty() {
    std::lock_guard<std::mutex> lock(mtx);
    return queue.empty();
  }
};