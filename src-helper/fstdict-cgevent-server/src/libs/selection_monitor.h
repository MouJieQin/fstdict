#pragma once
#include <string>

/// Initialize and start global mouse event listener
/// Runs event loop on calling thread (blocks)
bool startMouseEventListener();

/// Get the most recently captured selected text
std::string getLastSelectedText();
