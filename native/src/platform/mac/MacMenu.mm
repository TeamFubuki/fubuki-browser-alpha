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
  AddCommandItem(fileMenu, @"New Window", @"windows.create", @"n");
  NSMenuItem* privateWindow = AddCommandItem(fileMenu, @"New Private Window", @"windows.createPrivate", @"N");
  [privateWindow setKeyEquivalentModifierMask:NSEventModifierFlagShift | NSEventModifierFlagCommand];
  [fileMenu addItem:[NSMenuItem separatorItem]];
  AddCommandItem(fileMenu, @"Open Location", @"app.focusOmnibox", @"l");
  AddCommandItem(fileMenu, @"Home", @"tabs.home", @"");
  AddCommandItem(fileMenu, @"Close Tab", @"tabs.close", @"w");
  NSMenuItem* closeWindow = AddCommandItem(fileMenu, @"Close Window", @"windows.close", @"W");
  [closeWindow setKeyEquivalentModifierMask:NSEventModifierFlagShift | NSEventModifierFlagCommand];
  [fileMenu addItem:[NSMenuItem separatorItem]];
  AddCommandItem(fileMenu, @"Print...", @"page.print", @"p");
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
  [editMenu addItem:[NSMenuItem separatorItem]];
  AddCommandItem(editMenu, @"Find...", @"page.find", @"f");
  AddSubmenu(mainMenu, @"Edit", editMenu);

  NSMenu* viewMenu = [[NSMenu alloc] initWithTitle:@"View"];
  AddCommandItem(viewMenu, @"Reload Page", @"tabs.reload", @"r");
  AddCommandItem(viewMenu, @"Stop Loading", @"tabs.stop", @".");
  [viewMenu addItem:[NSMenuItem separatorItem]];
  AddCommandItem(viewMenu, @"Zoom In", @"page.zoomIn", @"+");
  AddCommandItem(viewMenu, @"Zoom Out", @"page.zoomOut", @"-");
  AddCommandItem(viewMenu, @"Actual Size", @"page.zoomReset", @"0");
  [viewMenu addItem:[NSMenuItem separatorItem]];
  AddCommandItem(viewMenu, @"View Source", @"page.viewSource", @"");
  [viewMenu addItem:[NSMenuItem separatorItem]];
  AddCommandItem(viewMenu, @"Developer Tools", @"app.openDevTools", @"");
  [viewMenu addItem:[NSMenuItem separatorItem]];
  AddItem(viewMenu, @"Enter Full Screen", @selector(toggleFullScreen:), @"f");
  [[viewMenu itemWithTitle:@"Enter Full Screen"] setKeyEquivalentModifierMask:NSEventModifierFlagControl | NSEventModifierFlagCommand];
  AddSubmenu(mainMenu, @"View", viewMenu);

  NSMenu* historyMenu = [[NSMenu alloc] initWithTitle:@"History"];
  AddCommandItem(historyMenu, @"Back", @"tabs.goBack", @"[");
  AddCommandItem(historyMenu, @"Forward", @"tabs.goForward", @"]");
  [historyMenu addItem:[NSMenuItem separatorItem]];
  AddCommandItem(historyMenu, @"Reopen Closed Tab", @"tabs.reopenClosed", @"T");
  AddCommandItem(historyMenu, @"Reopen Closed Window", @"windows.reopenClosed", @"");
  [historyMenu addItem:[NSMenuItem separatorItem]];
  AddCommandItem(historyMenu, @"Show History", @"app.openHistory", @"");
  AddSubmenu(mainMenu, @"History", historyMenu);

  NSMenu* bookmarksMenu = [[NSMenu alloc] initWithTitle:@"Bookmarks"];
  AddCommandItem(bookmarksMenu, @"Bookmark This Tab", @"bookmarks.addActive", @"d");
  AddCommandItem(bookmarksMenu, @"Show Bookmarks", @"app.openBookmarks", @"");
  AddSubmenu(mainMenu, @"Bookmarks", bookmarksMenu);

  NSMenu* toolsMenu = [[NSMenu alloc] initWithTitle:@"Tools"];
  AddCommandItem(toolsMenu, @"Downloads", @"app.openDownloads", @"");
  AddCommandItem(toolsMenu, @"Settings", @"app.openSettings", @"");
  AddCommandItem(toolsMenu, @"Debug", @"app.openDebug", @"");
  AddSubmenu(mainMenu, @"Tools", toolsMenu);

  NSMenu* windowMenu = [[NSMenu alloc] initWithTitle:@"Window"];
  AddItem(windowMenu, @"Minimize", @selector(performMiniaturize:), @"m");
  AddItem(windowMenu, @"Zoom", @selector(performZoom:), @"");
  [windowMenu addItem:[NSMenuItem separatorItem]];
  AddItem(windowMenu, @"Bring All to Front", @selector(arrangeInFront:), @"");
  [NSApp setWindowsMenu:windowMenu];
  AddSubmenu(mainMenu, @"Window", windowMenu);

  [NSApp setMainMenu:mainMenu];
}

NSMenu* FubukiCreateDockMenu() {
  NSMenu* dockMenu = [[NSMenu alloc] initWithTitle:@"Fubuki"];
  AddCommandItem(dockMenu, @"New Window", @"windows.create", @"");
  AddCommandItem(dockMenu, @"New Private Window", @"windows.createPrivate", @"");
  AddCommandItem(dockMenu, @"New Tab", @"tabs.create", @"");
  [dockMenu addItem:[NSMenuItem separatorItem]];
  AddCommandItem(dockMenu, @"Downloads", @"app.openDownloads", @"");
  AddCommandItem(dockMenu, @"History", @"app.openHistory", @"");
  AddCommandItem(dockMenu, @"Bookmarks", @"app.openBookmarks", @"");
  [dockMenu addItem:[NSMenuItem separatorItem]];
  AddCommandItem(dockMenu, @"Settings", @"app.openSettings", @"");
  return dockMenu;
}
