#import <Cocoa/Cocoa.h>

#import "include/cef_application_mac.h"
#include "include/wrapper/cef_helpers.h"

#include <string>

@interface FubukiApplication : NSApplication <CefAppProtocol> {
 @private
  BOOL handlingSendEvent_;
}
- (void)fubukiPerformCommand:(id)sender;
@end

namespace fubuki {
bool DispatchBrowserMenuCommand(const std::string& commandId);
}

@implementation FubukiApplication
- (BOOL)isHandlingSendEvent {
  return handlingSendEvent_;
}

- (void)setHandlingSendEvent:(BOOL)handlingSendEvent {
  handlingSendEvent_ = handlingSendEvent;
}

- (void)sendEvent:(NSEvent*)event {
  CefScopedSendingEvent sendingEventScoper;
  [super sendEvent:event];
}

- (void)fubukiPerformCommand:(id)sender {
  if (![sender respondsToSelector:@selector(representedObject)]) {
    return;
  }
  id command = [sender representedObject];
  if (![command isKindOfClass:[NSString class]]) {
    return;
  }
  fubuki::DispatchBrowserMenuCommand([(NSString*)command UTF8String]);
}
@end

@interface FubukiAppDelegate : NSObject <NSApplicationDelegate>
@end

void FubukiInstallBasicMenu();
NSMenu* FubukiCreateDockMenu();

namespace fubuki {

void InitializeMacApplication() {
  [FubukiApplication sharedApplication];
  static FubukiAppDelegate* delegate = [[FubukiAppDelegate alloc] init];
  [NSApp setDelegate:delegate];
  [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
  FubukiInstallBasicMenu();
  [NSApp activateIgnoringOtherApps:YES];
}

}  // namespace fubuki

@implementation FubukiAppDelegate
- (BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication*)sender {
  return NO;
}

- (NSMenu*)applicationDockMenu:(NSApplication*)sender {
  return FubukiCreateDockMenu();
}
@end
