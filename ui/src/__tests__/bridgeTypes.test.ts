import { describe, expect, it, expectTypeOf } from 'vitest';
import type {
  BridgeMethodMap,
  CommandId,
  EventMap,
  Settings,
  Tab,
  BrowserState,
} from '../bridge/fubuki';

describe('BridgeMethodMap types', () => {
  it('has correct param/result types for app.getState', () => {
    type Params = BridgeMethodMap['app.getState']['params'];
    type Result = BridgeMethodMap['app.getState']['result'];

    expectTypeOf<Params>().toEqualTypeOf<Record<string, never>>();
    expectTypeOf<Result>().toMatchTypeOf<{ windowId: string; tabs: Tab[] }>();
  });

  it('has correct param/result types for tabs.create', () => {
    type Params = BridgeMethodMap['tabs.create']['params'];
    type Result = BridgeMethodMap['tabs.create']['result'];

    expectTypeOf<Params>().toMatchTypeOf<{ url?: string; active?: boolean }>();
    expectTypeOf<Result>().toBeBoolean();
  });

  it('has correct param/result types for tabs.navigate', () => {
    type Params = BridgeMethodMap['tabs.navigate']['params'];
    type Result = BridgeMethodMap['tabs.navigate']['result'];

    expectTypeOf<Params>().toMatchTypeOf<{ tabId: string; input: string }>();
    expectTypeOf<Result>().toBeBoolean();
  });

  it('has correct param/result types for bookmarks.save', () => {
    type Params = BridgeMethodMap['bookmarks.save']['params'];
    type Result = BridgeMethodMap['bookmarks.save']['result'];

    expectTypeOf<Params>().toMatchTypeOf<{
      title: string;
      url: string;
      faviconUrl: string;
    }>();
    expectTypeOf<Result>().toBeBoolean();
  });

  it('has correct param/result types for settings.set', () => {
    type Params = BridgeMethodMap['settings.set']['params'];
    type Result = BridgeMethodMap['settings.set']['result'];

    expectTypeOf<Params>().toMatchTypeOf<{ key: string; value: string }>();
    expectTypeOf<Result>().toBeBoolean();
  });
});

describe('CommandId type', () => {
  it('includes all expected command ids', () => {
    const validIds: CommandId[] = [
      'tabs.create',
      'tabs.close',
      'tabs.reopenClosed',
      'tabs.duplicate',
      'tabs.pin',
      'tabs.unpin',
      'tabs.closeOther',
      'tabs.closeToRight',
      'tabs.moveToNewWindow',
      'tabs.reload',
      'tabs.stop',
      'tabs.goBack',
      'tabs.goForward',
      'tabs.home',
      'tabs.activateNext',
      'tabs.activatePrevious',
      'windows.create',
      'windows.createPrivate',
      'windows.close',
      'windows.reopenClosed',
      'app.focusOmnibox',
      'app.openSettings',
      'app.openHistory',
      'app.openDownloads',
      'app.openBookmarks',
      'app.openDebug',
      'app.toggleSidebar',
      'app.openDevTools',
      'page.find',
      'page.stopFinding',
      'page.zoomIn',
      'page.zoomOut',
      'page.zoomReset',
      'page.print',
      'page.viewSource',
      'bookmarks.addActive',
      'bookmarks.save',
      'bookmarks.remove',
    ];

    expect(validIds.length).toBeGreaterThan(0);
  });
});

describe('EventMap types', () => {
  it('has typed payloads for navigation events', () => {
    type StartedPayload = EventMap['navigation.started'];
    type FinishedPayload = EventMap['navigation.finished'];
    type FailedPayload = EventMap['navigation.failed'];

    expectTypeOf<StartedPayload>().toMatchTypeOf<{
      tabId: string;
      url: string;
    }>();
    expectTypeOf<FinishedPayload>().toMatchTypeOf<{
      tabId: string;
      url: string;
    }>();
    expectTypeOf<FailedPayload>().toMatchTypeOf<{
      tabId: string;
      url: string;
      errorText: string;
    }>();
  });

  it('has void payloads for state change events', () => {
    type TabsCreated = EventMap['tabs.created'];
    type TabsUpdated = EventMap['tabs.updated'];
    type BookmarkChanged = EventMap['bookmark.changed'];

    expectTypeOf<TabsCreated>().toBeVoid();
    expectTypeOf<TabsUpdated>().toBeVoid();
    expectTypeOf<BookmarkChanged>().toBeVoid();
  });
});

describe('Settings type', () => {
  it('has correct appearance union type', () => {
    type Appearance = Settings['appearance'];
    expectTypeOf<Appearance>().toEqualTypeOf<'system' | 'light' | 'dark'>();
  });

  it('has correct sidebarVisible union type', () => {
    type SidebarVisible = Settings['sidebarVisible'];
    expectTypeOf<SidebarVisible>().toEqualTypeOf<'show' | 'hide'>();
  });

  it('has correct newTabPage union type', () => {
    type NewTabPage = Settings['newTabPage'];
    expectTypeOf<NewTabPage>().toEqualTypeOf<'blank' | 'home'>();
  });
});
