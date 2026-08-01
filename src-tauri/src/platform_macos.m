#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#import <AudioToolbox/AudioToolbox.h>
#import <CoreAudio/CoreAudio.h>
#import <dispatch/dispatch.h>

#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <sys/statvfs.h>
#include <unistd.h>

static void RainOnMain(void (^block)(void)) {
  if ([NSThread isMainThread]) {
    block();
  } else {
    dispatch_sync(dispatch_get_main_queue(), block);
  }
}

static NSString *RainString(const uint8_t *bytes, size_t length) {
  if (bytes == NULL && length != 0) {
    return nil;
  }
  return [[NSString alloc] initWithBytes:bytes
                                  length:length
                                encoding:NSUTF8StringEncoding];
}

bool rain_accessibility_trusted(bool prompt) {
  if (!prompt) {
    return AXIsProcessTrusted();
  }
  NSDictionary *options = @{
    (__bridge NSString *)kAXTrustedCheckOptionPrompt: @YES,
  };
  return AXIsProcessTrustedWithOptions((__bridge CFDictionaryRef)options);
}

static pid_t RainFrontmostPid(void) {
  __block pid_t processId = 0;
  RainOnMain(^{
    processId = NSWorkspace.sharedWorkspace.frontmostApplication.processIdentifier;
  });
  return processId;
}

int32_t rain_frontmost_pid(void) {
  return RainFrontmostPid();
}

bool rain_target_is_active(int32_t processId) {
  return processId > 0 && RainFrontmostPid() == processId;
}

static AXUIElementRef RainFocusedWindow(pid_t processId) {
  AXUIElementRef application = AXUIElementCreateApplication(processId);
  if (application == NULL) {
    return NULL;
  }
  CFTypeRef value = NULL;
  AXError error = AXUIElementCopyAttributeValue(
      application, kAXFocusedWindowAttribute, &value);
  CFRelease(application);
  if (error != kAXErrorSuccess || value == NULL ||
      CFGetTypeID(value) != AXUIElementGetTypeID()) {
    if (value != NULL) {
      CFRelease(value);
    }
    return NULL;
  }
  return (AXUIElementRef)value;
}

static bool RainWindowGeometry(pid_t processId, CGPoint *position, CGSize *size) {
  AXUIElementRef window = RainFocusedWindow(processId);
  if (window == NULL) {
    return false;
  }
  CFTypeRef positionValue = NULL;
  CFTypeRef sizeValue = NULL;
  AXError positionError = AXUIElementCopyAttributeValue(
      window, kAXPositionAttribute, &positionValue);
  AXError sizeError = AXUIElementCopyAttributeValue(
      window, kAXSizeAttribute, &sizeValue);
  CFRelease(window);
  bool valid = positionError == kAXErrorSuccess &&
               sizeError == kAXErrorSuccess && positionValue != NULL &&
               sizeValue != NULL &&
               AXValueGetValue((AXValueRef)positionValue, kAXValueCGPointType,
                               position) &&
               AXValueGetValue((AXValueRef)sizeValue, kAXValueCGSizeType, size);
  if (positionValue != NULL) {
    CFRelease(positionValue);
  }
  if (sizeValue != NULL) {
    CFRelease(sizeValue);
  }
  return valid;
}

static NSScreen *RainScreenForProcess(pid_t processId) {
  CGPoint position = CGPointZero;
  CGSize size = CGSizeZero;
  bool hasGeometry = RainWindowGeometry(processId, &position, &size);
  CGPoint center = CGPointMake(position.x + size.width / 2.0,
                               position.y + size.height / 2.0);
  __block NSScreen *selected = nil;
  RainOnMain(^{
    if (hasGeometry) {
      for (NSScreen *screen in NSScreen.screens) {
        NSNumber *screenNumber = screen.deviceDescription[@"NSScreenNumber"];
        if (screenNumber == nil) {
          continue;
        }
        CGRect bounds = CGDisplayBounds(screenNumber.unsignedIntValue);
        if (CGRectContainsPoint(bounds, center)) {
          selected = screen;
          break;
        }
      }
    }
    if (selected == nil) {
      selected = NSScreen.mainScreen ?: NSScreen.screens.firstObject;
    }
  });
  return selected;
}

bool rain_target_is_fullscreen(int32_t processId) {
  AXUIElementRef window = RainFocusedWindow(processId);
  if (window == NULL) {
    return false;
  }
  CFTypeRef value = NULL;
  AXError error = AXUIElementCopyAttributeValue(
      window, CFSTR("AXFullScreen"), &value);
  CFRelease(window);
  bool fullscreen = error == kAXErrorSuccess && value != NULL &&
                    CFGetTypeID(value) == CFBooleanGetTypeID() &&
                    CFBooleanGetValue((CFBooleanRef)value);
  if (value != NULL) {
    CFRelease(value);
  }
  return fullscreen;
}

bool rain_target_work_area(int32_t processId, int32_t *left, int32_t *top,
                           int32_t *width, int32_t *height) {
  if (left == NULL || top == NULL || width == NULL || height == NULL) {
    return false;
  }
  NSScreen *screen = RainScreenForProcess(processId);
  if (screen == nil) {
    return false;
  }
  __block NSRect visible = NSZeroRect;
  __block CGFloat scale = 1.0;
  RainOnMain(^{
    visible = screen.visibleFrame;
    scale = screen.backingScaleFactor;
  });
  CGRect mainBounds = CGDisplayBounds(CGMainDisplayID());
  *left = (int32_t)llround(visible.origin.x * scale);
  *top = (int32_t)llround(
      (CGRectGetHeight(mainBounds) - NSMaxY(visible)) * scale);
  *width = (int32_t)llround(visible.size.width * scale);
  *height = (int32_t)llround(visible.size.height * scale);
  return *width > 0 && *height > 0;
}

static NSArray<NSPasteboardItem *> *RainPasteboardSnapshot(bool *complete) {
  NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
  NSMutableArray<NSPasteboardItem *> *snapshot = [NSMutableArray array];
  *complete = true;
  for (NSPasteboardItem *item in pasteboard.pasteboardItems ?: @[]) {
    NSPasteboardItem *copy = [[NSPasteboardItem alloc] init];
    for (NSPasteboardType type in item.types) {
      NSData *data = [item dataForType:type];
      if (data == nil || ![copy setData:[data copy] forType:type]) {
        *complete = false;
        return nil;
      }
    }
    [snapshot addObject:copy];
  }
  return [snapshot copy];
}

static bool RainWritePasteboardText(NSString *text, NSInteger *changeCount) {
  __block bool written = false;
  RainOnMain(^{
    NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
    [pasteboard clearContents];
    written = [pasteboard setString:text forType:NSPasteboardTypeString];
    if (written && changeCount != NULL) {
      *changeCount = pasteboard.changeCount;
    }
  });
  return written;
}

int32_t rain_copy_text(const uint8_t *bytes, size_t length) {
  NSString *text = RainString(bytes, length);
  return text != nil && RainWritePasteboardText(text, NULL) ? 0 : 1;
}

static bool RainCanPostKeyboardEvents(void) {
  if (!AXIsProcessTrusted()) {
    return false;
  }
  if (@available(macOS 10.15, *)) {
    return CGPreflightPostEventAccess();
  }
  return true;
}

static bool RainPostKey(CGKeyCode keyCode, bool down,
                        CGEventFlags flags) {
  CGEventRef event = CGEventCreateKeyboardEvent(NULL, keyCode, down);
  if (event == NULL) {
    return false;
  }
  CGEventSetFlags(event, flags);
  CGEventPost(kCGHIDEventTap, event);
  CFRelease(event);
  return true;
}

static bool RainPostPaste(void) {
  const CGKeyCode modifiers[] = {56, 60, 59, 62, 58, 61, 55, 54};
  for (size_t index = 0; index < sizeof(modifiers) / sizeof(modifiers[0]);
       index++) {
    if (CGEventSourceKeyState(kCGEventSourceStateCombinedSessionState,
                              modifiers[index])) {
      RainPostKey(modifiers[index], false, 0);
    }
  }
  return RainPostKey(9, true, kCGEventFlagMaskCommand) &&
         RainPostKey(9, false, kCGEventFlagMaskCommand);
}

int32_t rain_paste_text(const uint8_t *bytes, size_t length, int32_t processId,
                        bool restoreClipboard) {
  if (!rain_target_is_active(processId)) {
    return 1;
  }
  if (!RainCanPostKeyboardEvents()) {
    return 2;
  }
  NSString *text = RainString(bytes, length);
  if (text == nil) {
    return 4;
  }
  __block NSArray<NSPasteboardItem *> *snapshot = nil;
  __block bool snapshotComplete = true;
  if (restoreClipboard) {
    RainOnMain(^{
      snapshot = RainPasteboardSnapshot(&snapshotComplete);
    });
    if (!snapshotComplete || snapshot == nil) {
      return 3;
    }
  }
  NSInteger writtenChangeCount = 0;
  if (!RainWritePasteboardText(text, &writtenChangeCount)) {
    return 4;
  }
  if (!rain_target_is_active(processId)) {
    return 1;
  }
  if (!RainPostPaste()) {
    return 5;
  }
  usleep(300000);
  if (restoreClipboard) {
    __block bool restored = true;
    RainOnMain(^{
      NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
      if (pasteboard.changeCount == writtenChangeCount) {
        [pasteboard clearContents];
        if (snapshot.count > 0 && ![pasteboard writeObjects:snapshot]) {
          restored = false;
        }
      }
    });
    if (!restored) {
      return 6;
    }
  }
  return 0;
}

int32_t rain_type_text(const uint8_t *bytes, size_t length, int32_t processId) {
  if (!rain_target_is_active(processId)) {
    return 1;
  }
  if (!RainCanPostKeyboardEvents()) {
    return 2;
  }
  NSString *text = RainString(bytes, length);
  if (text == nil || text.length == 0) {
    return text == nil ? 3 : 0;
  }
  NSUInteger offset = 0;
  while (offset < text.length) {
    if (!rain_target_is_active(processId)) {
      return 1;
    }
    NSUInteger chunkLength = MIN((NSUInteger)32, text.length - offset);
    if (offset + chunkLength < text.length) {
      unichar last = [text characterAtIndex:offset + chunkLength - 1];
      if (CFStringIsSurrogateHighCharacter(last)) {
        chunkLength -= 1;
      }
    }
    UniChar buffer[32];
    [text getCharacters:buffer range:NSMakeRange(offset, chunkLength)];
    CGEventRef down = CGEventCreateKeyboardEvent(NULL, 0, true);
    CGEventRef up = CGEventCreateKeyboardEvent(NULL, 0, false);
    if (down == NULL || up == NULL) {
      if (down != NULL) CFRelease(down);
      if (up != NULL) CFRelease(up);
      return 3;
    }
    CGEventKeyboardSetUnicodeString(down, chunkLength, buffer);
    CGEventKeyboardSetUnicodeString(up, chunkLength, buffer);
    CGEventPost(kCGHIDEventTap, down);
    CGEventPost(kCGHIDEventTap, up);
    CFRelease(down);
    CFRelease(up);
    offset += chunkLength;
  }
  return 0;
}

bool rain_show_window(void *windowPointer, bool interactive) {
  if (windowPointer == NULL) {
    return false;
  }
  NSWindow *window = (__bridge NSWindow *)windowPointer;
  RainOnMain(^{
    window.level = NSStatusWindowLevel;
    window.collectionBehavior |= NSWindowCollectionBehaviorCanJoinAllSpaces |
                                 NSWindowCollectionBehaviorFullScreenAuxiliary |
                                 NSWindowCollectionBehaviorStationary;
    window.hidesOnDeactivate = NO;
    window.ignoresMouseEvents = interactive ? NO : YES;
    [window orderFrontRegardless];
  });
  return true;
}

void rain_hide_window(void *windowPointer) {
  if (windowPointer == NULL) {
    return;
  }
  NSWindow *window = (__bridge NSWindow *)windowPointer;
  RainOnMain(^{
    [window orderOut:nil];
  });
}

void rain_play_sound(const uint8_t *bytes, size_t length) {
  NSString *kind = RainString(bytes, length) ?: @"start";
  RainOnMain(^{
    NSString *name = [kind isEqualToString:@"error"] ? @"Basso" :
                     [kind isEqualToString:@"stop"] ? @"Pop" : @"Tink";
    [[NSSound soundNamed:name] play];
  });
}

bool rain_confirm_exit(bool english) {
  __block bool confirmed = false;
  RainOnMain(^{
    NSAlert *alert = [[NSAlert alloc] init];
    alert.messageText = @"雨音输入法";
    alert.informativeText = english
        ? @"Recording or recognition is active. Exit Rain and cancel the current task?"
        : @"正在录音或识别。确定退出 Rain 并取消当前任务吗？";
    [alert addButtonWithTitle:english ? @"Exit" : @"退出"];
    [alert addButtonWithTitle:english ? @"Cancel" : @"取消"];
    alert.alertStyle = NSAlertStyleWarning;
    confirmed = [alert runModal] == NSAlertFirstButtonReturn;
  });
  return confirmed;
}

bool rain_system_prefers_english(void) {
  __block bool english = true;
  RainOnMain(^{
    NSString *language = NSLocale.preferredLanguages.firstObject ?: @"en";
    english = ![language.lowercaseString hasPrefix:@"zh"];
  });
  return english;
}

bool rain_free_disk_space(const uint8_t *bytes, size_t length, uint64_t *output) {
  if (bytes == NULL || output == NULL) {
    return false;
  }
  char *path = calloc(length + 1, 1);
  if (path == NULL) {
    return false;
  }
  memcpy(path, bytes, length);
  struct statvfs info;
  int status = statvfs(path, &info);
  free(path);
  if (status != 0) {
    return false;
  }
  *output = (uint64_t)info.f_bavail * (uint64_t)info.f_frsize;
  return true;
}

typedef struct {
  AudioDeviceID device;
  UInt32 count;
  AudioObjectPropertyElement elements[2];
  Float32 volumes[2];
} RainAudioDuckToken;

static bool RainVolume(AudioDeviceID device, AudioObjectPropertyElement element,
                       Float32 *value, bool write) {
  AudioObjectPropertyAddress address = {
    kAudioDevicePropertyVolumeScalar,
    kAudioDevicePropertyScopeOutput,
    element,
  };
  if (!AudioObjectHasProperty(device, &address)) {
    return false;
  }
  Boolean settable = false;
  if (AudioObjectIsPropertySettable(device, &address, &settable) != noErr ||
      !settable) {
    return false;
  }
  UInt32 size = sizeof(Float32);
  OSStatus status = write
      ? AudioObjectSetPropertyData(device, &address, 0, NULL, size, value)
      : AudioObjectGetPropertyData(device, &address, 0, NULL, &size, value);
  return status == noErr;
}

void *rain_duck_system_audio(void) {
  AudioDeviceID device = kAudioObjectUnknown;
  AudioObjectPropertyAddress defaultOutput = {
    kAudioHardwarePropertyDefaultOutputDevice,
    kAudioObjectPropertyScopeGlobal,
    kAudioObjectPropertyElementMain,
  };
  UInt32 size = sizeof(device);
  if (AudioObjectGetPropertyData(kAudioObjectSystemObject, &defaultOutput, 0,
                                 NULL, &size, &device) != noErr ||
      device == kAudioObjectUnknown) {
    return NULL;
  }
  RainAudioDuckToken *token = calloc(1, sizeof(RainAudioDuckToken));
  if (token == NULL) {
    return NULL;
  }
  token->device = device;
  Float32 master = 0;
  if (RainVolume(device, kAudioObjectPropertyElementMain, &master, false)) {
    token->count = 1;
    token->elements[0] = kAudioObjectPropertyElementMain;
    token->volumes[0] = master;
  } else {
    for (AudioObjectPropertyElement channel = 1; channel <= 2; channel++) {
      Float32 volume = 0;
      if (RainVolume(device, channel, &volume, false)) {
        token->elements[token->count] = channel;
        token->volumes[token->count] = volume;
        token->count++;
      }
    }
  }
  if (token->count == 0) {
    free(token);
    return NULL;
  }
  for (UInt32 index = 0; index < token->count; index++) {
    Float32 ducked = fmaxf(0.0f, fminf(1.0f, token->volumes[index] * 0.2f));
    if (!RainVolume(device, token->elements[index], &ducked, true)) {
      for (UInt32 restore = 0; restore < index; restore++) {
        RainVolume(device, token->elements[restore], &token->volumes[restore], true);
      }
      free(token);
      return NULL;
    }
  }
  return token;
}

void rain_restore_system_audio(void *tokenPointer) {
  RainAudioDuckToken *token = tokenPointer;
  if (token == NULL) {
    return;
  }
  for (UInt32 index = 0; index < token->count; index++) {
    RainVolume(token->device, token->elements[index], &token->volumes[index], true);
  }
  free(token);
}

void rain_kill_process_group(int32_t processGroup) {
  if (processGroup > 0) {
    kill(-processGroup, SIGTERM);
  }
}
