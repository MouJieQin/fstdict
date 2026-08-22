#include "clipboard_manager.h"
#import <Cocoa/Cocoa.h>

std::string getClipboardText() {
  NSPasteboard *pasteboard = [NSPasteboard generalPasteboard];
  NSString *content = [pasteboard stringForType:NSPasteboardTypeString];
  return content ? std::string([content UTF8String]) : "";
}

void setClipboardText(const std::string &text) {
  NSPasteboard *pasteboard = [NSPasteboard generalPasteboard];
  [pasteboard clearContents];
  [pasteboard setString:[NSString stringWithUTF8String:text.c_str()]
                forType:NSPasteboardTypeString];
}

void simulateCopyShortcut() {
  // Virtual key code for 'C' is 8
  CGEventRef keyDown = CGEventCreateKeyboardEvent(NULL, (CGKeyCode)8, true);
  CGEventSetFlags(keyDown, kCGEventFlagMaskCommand);
  CGEventPost(kCGHIDEventTap, keyDown);
  CFRelease(keyDown);

  CGEventRef keyUp = CGEventCreateKeyboardEvent(NULL, (CGKeyCode)8, false);
  CGEventSetFlags(keyUp, kCGEventFlagMaskCommand);
  CGEventPost(kCGHIDEventTap, keyUp);
  CFRelease(keyUp);
}
