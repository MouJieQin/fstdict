#pragma once
#include <functional>
#include <string>

/// Callback signature for async HTTP requests
/// @param ok True if request succeeded
/// @param bodyOrErr Response body on success, error message on failure
using HttpCallback = std::function<void(bool ok, const std::string &bodyOrErr)>;

/// Perform an async HTTP GET request using macOS native NSURLSession
/// @param url Target URL
/// @param timeoutSec Request timeout in seconds
/// @param callback Completion callback
void httpGetAsync(const std::string &url, int timeoutSec,
                  HttpCallback callback);
