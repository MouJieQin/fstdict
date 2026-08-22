#pragma once
#include <string>

/// Get current plain-text content from system clipboard
std::string getClipboardText();

/// Replace system clipboard content with plain text
void setClipboardText(const std::string &text);

/// Simulate Cmd+C keyboard shortcut to trigger copy in active app
void simulateCopyShortcut();
