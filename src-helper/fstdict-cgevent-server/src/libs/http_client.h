// http_client.h
#pragma once
#include <functional>
#include <string>

// 回调签名：success(http_body) | error(message)
using HttpCallback =
    std::function<void(bool ok, const std::string &body_or_err)>;

/// 异步GET请求（macOS NSURLSession）
void http_get_async(const std::string &url, int timeout_sec, HttpCallback cb);