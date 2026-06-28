#import <Cocoa/Cocoa.h>

void FubukiInstallBasicMenu() {
  NSMenu* mainMenu = [[NSMenu alloc] initWithTitle:@"Fubuki"];
  NSMenuItem* appItem = [[NSMenuItem alloc] initWithTitle:@"Fubuki" action:nil keyEquivalent:@""];
  [mainMenu addItem:appItem];
  NSMenu* appMenu = [[NSMenu alloc] initWithTitle:@"Fubuki"];
  [appMenu addItemWithTitle:@"Quit Fubuki Browser Alpha" action:@selector(terminate:) keyEquivalent:@"q"];
  [appItem setSubmenu:appMenu];
  [NSApp setMainMenu:mainMenu];
}
