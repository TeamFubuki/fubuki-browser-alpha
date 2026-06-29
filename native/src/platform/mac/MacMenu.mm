#import <Cocoa/Cocoa.h>

namespace {

NSMenuItem* AddItem(NSMenu* menu, NSString* title, SEL action, NSString* key) {
  return [menu addItemWithTitle:title action:action keyEquivalent:key];
}

NSMenuItem* AddCommandItem(NSMenu* menu, NSString* title, NSString* commandId, NSString* key) {
  NSMenuItem* item = AddItem(menu, title, @selector(fubukiPerformCommand:), key);
  [item setRepresentedObject:commandId];
  return item;
}

void AddSubmenu(NSMenu* mainMenu, NSString* title, NSMenu* submenu) {
  NSMenuItem* item = [[NSMenuItem alloc] initWithTitle:title action:nil keyEquivalent:@""];
  [item setSubmenu:submenu];
  [mainMenu addItem:item];
}

}  // namespace

void FubukiInstallBasicMenu() {
  NSMenu* mainMenu = [[NSMenu alloc] initWithTitle:@"Fubuki"];

  NSMenuItem* appItem = [[NSMenuItem alloc] initWithTitle:@"Fubuki" action:nil keyEquivalent:@""];
  [mainMenu addItem:appItem];
  NSMenu* appMenu = [[NSMenu alloc] initWithTitle:@"Fubuki"];
  [appMenu addItemWithTitle:@"About Fubuki Browser Alpha" action:@selector(orderFrontStandardAboutPanel:) keyEquivalent:@""];
  [appMenu addItem:[NSMenuItem separatorItem]];
  [appMenu addItemWithTitle:@"Hide Fubuki Browser Alpha" action:@selector(hide:) keyEquivalent:@"h"];
  NSMenuItem* hideOthers = [appMenu addItemWithTitle:@"Hide Others" action:@selector(hideOtherApplications:) keyEquivalent:@"h"];
  [hideOthers setKeyEquivalentModifierMask:NSEventModifierFlagOption | NSEventModifierFlagCommand];
  [appMenu addItemWithTitle:@"Show All" action:@selector(unhideAllApplications:) keyEquivalent:@""];
  [appMenu addItem:[NSMenuItem separatorItem]];
  AddCommandItem(appMenu, @"Settings...", @"app.openSettings", @",");
  [appMenu addItem:[NSMenuItem separatorItem]];
  [appMenu addItemWithTitle:@"Quit Fubuki Browser Alpha" action:@selector(terminate:) keyEquivalent:@"q"];
  [appItem setSubmenu:appMenu];

  NSMenu* fileMenu = [[NSMenu alloc] initWithTitle:@"File"];
  AddCommandItem(fileMenu, @"New Tab", @"tabs.create", @"t");
  AddCommandItem(fileMenu, @"Open Location", @"app.focusOmnibox", @"l");
  AddCommandItem(fileMenu, @"Settings", @"app.openSettings", @"");
  AddCommandItem(fileMenu, @"Close Tab", @"tabs.close", @"w");
  [fileMenu addItem:[NSMenuItem separatorItem]];
  AddItem(fileMenu, @"Close Window", @selector(performClose:), @"W");
  AddSubmenu(mainMenu, @"File", fileMenu);

  NSMenu* editMenu = [[NSMenu alloc] initWithTitle:@"Edit"];
  AddItem(editMenu, @"Undo", @selector(undo:), @"z");
  AddItem(editMenu, @"Redo", @selector(redo:), @"Z");
  [editMenu addItem:[NSMenuItem separatorItem]];
  AddItem(editMenu, @"Cut", @selector(cut:), @"x");
  AddItem(editMenu, @"Copy", @selector(copy:), @"c");
  AddItem(editMenu, @"Paste", @selector(paste:), @"v");
  AddItem(editMenu, @"Paste and Match Style", @selector(pasteAsPlainText:), @"V");
  AddItem(editMenu, @"Delete", @selector(delete:), @"");
  AddItem(editMenu, @"Select All", @selector(selectAll:), @"a");
  AddSubmenu(mainMenu, @"Edit", editMenu);

  NSMenu* viewMenu = [[NSMenu alloc] initWithTitle:@"View"];
  AddCommandItem(viewMenu, @"Reload Page", @"tabs.reload", @"r");
  AddCommandItem(viewMenu, @"Stop Loading", @"tabs.stop", @".");
  [viewMenu addItem:[NSMenuItem separatorItem]];
  AddItem(viewMenu, @"Enter Full Screen", @selector(toggleFullScreen:), @"f");
  [[viewMenu itemWithTitle:@"Enter Full Screen"] setKeyEquivalentModifierMask:NSEventModifierFlagControl | NSEventModifierFlagCommand];
  AddSubmenu(mainMenu, @"View", viewMenu);

  NSMenu* historyMenu = [[NSMenu alloc] initWithTitle:@"History"];
  AddCommandItem(historyMenu, @"Back", @"tabs.goBack", @"[");
  AddCommandItem(historyMenu, @"Forward", @"tabs.goForward", @"]");
  AddSubmenu(mainMenu, @"History", historyMenu);

  NSMenu* windowMenu = [[NSMenu alloc] initWithTitle:@"Window"];
  AddItem(windowMenu, @"Minimize", @selector(performMiniaturize:), @"m");
  AddItem(windowMenu, @"Zoom", @selector(performZoom:), @"");
  [windowMenu addItem:[NSMenuItem separatorItem]];
  AddItem(windowMenu, @"Bring All to Front", @selector(arrangeInFront:), @"");
  [NSApp setWindowsMenu:windowMenu];
  AddSubmenu(mainMenu, @"Window", windowMenu);

  [NSApp setMainMenu:mainMenu];
}
