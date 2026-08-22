#include "accessibility_manager.h"
#include "logger.h"
#import <Cocoa/Cocoa.h>

bool ensureAccessibilityPermissions() {
  NSDictionary *options = @{(id)kAXTrustedCheckOptionPrompt : @YES};
  bool trusted =
      AXIsProcessTrustedWithOptions((__bridge CFDictionaryRef)options);

  if (!trusted) {
    LOG_CRITICAL("Accessibility permission denied. "
                 "Please enable this app in System Settings > Privacy & "
                 "Security > Accessibility.");
  }
  return trusted;
}
