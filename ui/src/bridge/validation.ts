import type {
  BookmarkRecord,
  BrowserCommand,
  BrowserState,
  DownloadRecord,
  EventMap,
  FrostAppState,
  FrostTabState,
  FrostWindowState,
  HistoryRecord,
  PermissionRecord,
  Settings,
  TabSnapshot,
  WindowSnapshot,
} from './fubuki';

type JsonRecord = Record<string, unknown>;
type Validator = (value: unknown, path: string) => unknown;

const settingsKeys = new Set<keyof Settings>([
  'homepage',
  'searchEngine',
  'customSearchUrl',
  'theme',
  'appearance',
  'sidebarVisible',
  'sidebarWidth',
  'newTabPage',
  'homeUrl',
  'language',
  'defaultZoomLevel',
  'startupBehavior',
  'downloadDirectory',
  'askBeforeDownload',
  'closeWindowWithLastTab',
]);

function fail(path: string, expected: string): never {
  throw new Error(`${path} must be ${expected}`);
}

function record(value: unknown, path: string): JsonRecord {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    fail(path, 'an object');
  }
  return value as JsonRecord;
}

function string(value: unknown, path: string, nonEmpty = false): string {
  if (typeof value !== 'string' || (nonEmpty && value.trim().length === 0)) {
    fail(path, nonEmpty ? 'a non-empty string' : 'a string');
  }
  return value;
}

function boolean(value: unknown, path: string): boolean {
  if (typeof value !== 'boolean') fail(path, 'a boolean');
  return value;
}

function finiteNumber(value: unknown, path: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    fail(path, 'a finite number');
  }
  return value;
}

function integer(value: unknown, path: string, minimum = 0): number {
  const result = finiteNumber(value, path);
  if (!Number.isInteger(result) || result < minimum) {
    fail(path, `an integer greater than or equal to ${minimum}`);
  }
  return result;
}

function nullableId(value: unknown, path: string): string | null {
  return value === null ? null : string(value, path, true);
}

function array<T>(
  value: unknown,
  path: string,
  itemValidator: (item: unknown, itemPath: string) => T,
): T[] {
  if (!Array.isArray(value)) fail(path, 'an array');
  return value.map((item, index) => itemValidator(item, `${path}[${index}]`));
}

function validateTab(value: unknown, path: string): FrostTabState {
  const item = record(value, path);
  return {
    id: string(item.id, `${path}.id`, true),
    windowId: string(item.windowId, `${path}.windowId`, true),
    title: string(item.title, `${path}.title`),
    url: string(item.url, `${path}.url`),
    faviconUrl: string(item.faviconUrl, `${path}.faviconUrl`),
    errorText: string(item.errorText, `${path}.errorText`),
    zoomLevel: finiteNumber(item.zoomLevel, `${path}.zoomLevel`),
    isLoading: boolean(item.isLoading, `${path}.isLoading`),
    canGoBack: boolean(item.canGoBack, `${path}.canGoBack`),
    canGoForward: boolean(item.canGoForward, `${path}.canGoForward`),
    isActive: boolean(item.isActive, `${path}.isActive`),
    isPinned: boolean(item.isPinned, `${path}.isPinned`),
  };
}

function validateFrostWindow(value: unknown, path: string): FrostWindowState {
  const item = record(value, path);
  return {
    id: string(item.id, `${path}.id`, true),
    activeTabId: nullableId(item.activeTabId, `${path}.activeTabId`),
    isPrivate: boolean(item.isPrivate, `${path}.isPrivate`),
    tabIds: array(item.tabIds, `${path}.tabIds`, (id, idPath) =>
      string(id, idPath, true),
    ),
  };
}

function validateTabSnapshot(value: unknown, path: string): TabSnapshot {
  const item = record(value, path);
  return {
    title: string(item.title, `${path}.title`),
    url: string(item.url, `${path}.url`),
    faviconUrl: string(item.faviconUrl, `${path}.faviconUrl`),
    pinned: boolean(item.pinned, `${path}.pinned`),
    active: boolean(item.active, `${path}.active`),
  };
}

function validateWindowSnapshot(value: unknown, path: string): WindowSnapshot {
  const item = record(value, path);
  return {
    id: string(item.id, `${path}.id`, true),
    ...(item.private === undefined
      ? {}
      : { private: boolean(item.private, `${path}.private`) }),
    activeTabId: string(item.activeTabId, `${path}.activeTabId`),
    tabs: array(item.tabs, `${path}.tabs`, validateTabSnapshot),
  };
}

function validateHistory(value: unknown, path: string): HistoryRecord {
  const item = record(value, path);
  return {
    title: string(item.title, `${path}.title`),
    url: string(item.url, `${path}.url`, true),
    faviconUrl: string(item.faviconUrl, `${path}.faviconUrl`),
    createdAt: string(item.createdAt, `${path}.createdAt`),
  };
}

function validateBookmark(value: unknown, path: string): BookmarkRecord {
  return validateHistory(value, path);
}

function validateDownload(value: unknown, path: string): DownloadRecord {
  const item = record(value, path);
  const percent = finiteNumber(item.percent, `${path}.percent`);
  return {
    url: string(item.url, `${path}.url`, true),
    path: string(item.path, `${path}.path`),
    state: string(item.state, `${path}.state`, true),
    percent: Math.min(100, Math.max(0, percent)),
    createdAt: string(item.createdAt, `${path}.createdAt`),
  };
}

function validatePermission(value: unknown, path: string): PermissionRecord {
  const item = record(value, path);
  return {
    origin: string(item.origin, `${path}.origin`, true),
    permission: string(item.permission, `${path}.permission`, true),
    value: string(item.value, `${path}.value`),
    createdAt: string(item.createdAt, `${path}.createdAt`),
  };
}

function validateSettings(value: unknown, path: string): Partial<Settings> {
  const item = record(value, path);
  const result: Partial<Settings> = {};
  for (const [key, setting] of Object.entries(item)) {
    if (!settingsKeys.has(key as keyof Settings)) continue;
    (result as Record<string, string>)[key] = string(setting, `${path}.${key}`);
  }
  if (
    result.appearance !== undefined &&
    !['system', 'light', 'dark'].includes(result.appearance)
  ) {
    fail(`${path}.appearance`, 'one of system, light, or dark');
  }
  if (
    result.sidebarVisible !== undefined &&
    !['show', 'hide'].includes(result.sidebarVisible)
  ) {
    fail(`${path}.sidebarVisible`, 'one of show or hide');
  }
  if (
    result.newTabPage !== undefined &&
    !['blank', 'home'].includes(result.newTabPage)
  ) {
    fail(`${path}.newTabPage`, 'one of blank or home');
  }
  return result;
}

function validateFrostAppState(value: unknown, path: string): FrostAppState {
  const item = record(value, path);
  const activeWindowId = nullableId(
    item.activeWindowId,
    `${path}.activeWindowId`,
  );
  return {
    protocolVersion: integer(item.protocolVersion, `${path}.protocolVersion`),
    ...(item.currentWindowId === undefined
      ? {}
      : {
          currentWindowId: string(
            item.currentWindowId,
            `${path}.currentWindowId`,
            true,
          ),
        }),
    activeWindowId,
    windows: array(item.windows, `${path}.windows`, validateFrostWindow),
    tabs: array(item.tabs, `${path}.tabs`, validateTab),
    ...(item.history === undefined
      ? {}
      : { history: array(item.history, `${path}.history`, validateHistory) }),
    ...(item.bookmarks === undefined
      ? {}
      : {
          bookmarks: array(
            item.bookmarks,
            `${path}.bookmarks`,
            validateBookmark,
          ),
        }),
    ...(item.downloads === undefined
      ? {}
      : {
          downloads: array(
            item.downloads,
            `${path}.downloads`,
            validateDownload,
          ),
        }),
    ...(item.permissions === undefined
      ? {}
      : {
          permissions: array(
            item.permissions,
            `${path}.permissions`,
            validatePermission,
          ),
        }),
    ...(item.settings === undefined
      ? {}
      : { settings: validateSettings(item.settings, `${path}.settings`) }),
  };
}

function validateBrowserState(value: unknown, path: string): BrowserState {
  const item = record(value, path);
  const settings = validateSettings(item.settings, `${path}.settings`);
  if (Object.keys(settings).length !== settingsKeys.size) {
    fail(`${path}.settings`, 'a complete settings object');
  }
  return {
    bridgeVersion: string(item.bridgeVersion, `${path}.bridgeVersion`, true),
    windowId: string(item.windowId, `${path}.windowId`),
    isPrivate: boolean(item.isPrivate, `${path}.isPrivate`),
    activeTabId: string(item.activeTabId, `${path}.activeTabId`),
    tabs: array(item.tabs, `${path}.tabs`, validateTab),
    windows: array(item.windows, `${path}.windows`, validateWindowSnapshot),
    history: array(item.history, `${path}.history`, validateHistory),
    bookmarks: array(item.bookmarks, `${path}.bookmarks`, validateBookmark),
    downloads: array(item.downloads, `${path}.downloads`, validateDownload),
    permissions: array(
      item.permissions,
      `${path}.permissions`,
      validatePermission,
    ),
    logs: array(item.logs, `${path}.logs`, (entry, entryPath) => {
      const log = record(entry, entryPath);
      return {
        level: string(log.level, `${entryPath}.level`),
        message: string(log.message, `${entryPath}.message`),
        createdAt: string(log.createdAt, `${entryPath}.createdAt`),
      };
    }),
    commands: array(item.commands, `${path}.commands`, validateCommand),
    recentEvents: array(
      item.recentEvents,
      `${path}.recentEvents`,
      (entry, entryPath) => {
        const event = record(entry, entryPath);
        return {
          name: string(event.name, `${entryPath}.name`),
          windowId: string(event.windowId, `${entryPath}.windowId`),
          tabId: string(event.tabId, `${entryPath}.tabId`),
          message: string(event.message, `${entryPath}.message`),
        };
      },
    ),
    settings: settings as Settings,
    profilePath: string(item.profilePath, `${path}.profilePath`),
  };
}

function validateCommand(value: unknown, path: string): BrowserCommand {
  const item = record(value, path);
  return {
    id: string(item.id, `${path}.id`, true),
    title: string(item.title, `${path}.title`),
    category: string(item.category, `${path}.category`),
    shortcut: string(item.shortcut, `${path}.shortcut`),
  };
}

function validateSnapshot(value: unknown, path: string) {
  const item = record(value, path);
  return 'protocolVersion' in item
    ? validateFrostAppState(item, path)
    : validateBrowserState(item, path);
}

const booleanMethods = new Set([
  'tabs.create',
  'tabs.pin',
  'tabs.unpin',
  'tabs.duplicate',
  'tabs.reopenClosed',
  'tabs.closeOther',
  'tabs.closeToRight',
  'tabs.moveToNewWindow',
  'tabs.activateNext',
  'tabs.activatePrevious',
  'tabs.navigate',
  'tabs.activate',
  'tabs.close',
  'tabs.reload',
  'tabs.stop',
  'tabs.goBack',
  'tabs.goForward',
  'tabs.move',
  'tabs.home',
  'windows.create',
  'windows.createPrivate',
  'windows.close',
  'windows.reopenClosed',
  'windows.reopenClosedPrivate',
  'bookmarks.save',
  'bookmarks.remove',
  'bookmarks.clear',
  'history.remove',
  'history.clearRange',
  'history.clear',
  'downloads.remove',
  'downloads.clear',
  'downloads.open',
  'downloads.reveal',
  'settings.set',
  'settings.reset',
  'ui.setSidebarWidth',
  'ui.setOverlayActive',
]);

const responseValidators = new Map<string, Validator>([
  ['app.getState', validateBrowserState],
  ['app.snapshot', validateSnapshot],
  ['commands.list', (value, path) => array(value, path, validateCommand)],
  ['tabs.list', (value, path) => array(value, path, validateTab)],
  [
    'windows.list',
    (value, path) =>
      array(value, path, (item, itemPath) => {
        const object = record(item, itemPath);
        return 'tabIds' in object
          ? validateFrostWindow(object, itemPath)
          : validateWindowSnapshot(object, itemPath);
      }),
  ],
  ['bookmarks.list', (value, path) => array(value, path, validateBookmark)],
  ['history.list', (value, path) => array(value, path, validateHistory)],
  ['downloads.list', (value, path) => array(value, path, validateDownload)],
  [
    'settings.get',
    (value, path) => (value === null ? null : string(value, path)),
  ],
]);

export function validateBridgeResponse(
  method: string,
  value: unknown,
): unknown {
  try {
    if (booleanMethods.has(method)) return boolean(value, 'response');
    return responseValidators.get(method)?.(value, 'response') ?? value;
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    throw new Error(`Invalid response for "${method}": ${reason}`);
  }
}

function validateVoid(value: unknown, path: string): undefined {
  if (value === undefined || value === null) return undefined;
  const item = record(value, path);
  if (Object.keys(item).length > 0) fail(path, 'empty');
  return undefined;
}

function validateIdEvent(value: unknown, path: string, key: string) {
  const item = record(value, path);
  return { [key]: string(item[key], `${path}.${key}`, true) };
}

function validateTabPatch(value: unknown, path: string) {
  const item = record(value, path);
  const result: JsonRecord = {
    tabId: string(item.tabId, `${path}.tabId`, true),
  };
  const stringFields = ['title', 'url', 'faviconUrl', 'errorText', 'windowId'];
  const booleanFields = [
    'isLoading',
    'canGoBack',
    'canGoForward',
    'isActive',
    'isPinned',
  ];
  for (const key of stringFields) {
    if (item[key] !== undefined)
      result[key] = string(item[key], `${path}.${key}`);
  }
  for (const key of booleanFields) {
    if (item[key] !== undefined)
      result[key] = boolean(item[key], `${path}.${key}`);
  }
  if (item.zoomLevel !== undefined) {
    result.zoomLevel = finiteNumber(item.zoomLevel, `${path}.zoomLevel`);
  }
  return result;
}

function validateNavigation(value: unknown, path: string, failed = false) {
  const item = record(value, path);
  return {
    tabId: string(item.tabId, `${path}.tabId`, true),
    url: string(item.url, `${path}.url`),
    ...(failed
      ? { errorText: string(item.errorText, `${path}.errorText`) }
      : {}),
  };
}

function validateOptionalUrl(value: unknown, path: string) {
  if (value === undefined || value === null) return undefined;
  const item = record(value, path);
  return item.url === undefined ? {} : { url: string(item.url, `${path}.url`) };
}

function validatePermissionEvent(value: unknown, path: string) {
  const item = record(value, path);
  return {
    origin: string(item.origin, `${path}.origin`, true),
    permission: string(item.permission, `${path}.permission`, true),
  };
}

const externalCapabilities = new Set([
  'read_state',
  'tab_control',
  'navigation',
  'bookmarks',
  'history',
  'downloads',
  'debug',
]);

function validateExternalCapability(value: unknown, path: string) {
  const capability = string(value, path, true);
  if (!externalCapabilities.has(capability)) {
    fail(path, 'a supported external capability');
  }
  return capability;
}

function validateDownloadEvent(value: unknown, path: string) {
  if (value === undefined || value === null) return undefined;
  const item = record(value, path);
  const result: JsonRecord = {};
  for (const key of ['url', 'path', 'state', 'createdAt']) {
    if (item[key] !== undefined)
      result[key] = string(item[key], `${path}.${key}`);
  }
  if (item.percent !== undefined) {
    result.percent = Math.min(
      100,
      Math.max(0, finiteNumber(item.percent, `${path}.percent`)),
    );
  }
  return result;
}

const eventValidators = new Map<string, Validator>([
  ['tab.created', validateTab],
  ['tab.updated', validateTabPatch],
  ['tab.closed', (value, path) => validateIdEvent(value, path, 'tabId')],
  ['tab.activated', (value, path) => validateIdEvent(value, path, 'tabId')],
  [
    'tab.moved',
    (value, path) => {
      const item = record(value, path);
      return {
        tabId: string(item.tabId, `${path}.tabId`, true),
        fromWindowId: string(item.fromWindowId, `${path}.fromWindowId`, true),
        toWindowId: string(item.toWindowId, `${path}.toWindowId`, true),
        toIndex: integer(item.toIndex, `${path}.toIndex`),
      };
    },
  ],
  ['navigation.started', (value, path) => validateNavigation(value, path)],
  ['navigation.finished', (value, path) => validateNavigation(value, path)],
  ['navigation.failed', (value, path) => validateNavigation(value, path, true)],
  [
    'setting.changed',
    (value, path) => {
      const item = record(value, path);
      return {
        key: string(item.key, `${path}.key`, true),
        value: string(item.value, `${path}.value`),
      };
    },
  ],
  ['bookmark.changed', validateOptionalUrl],
  ['history.changed', validateOptionalUrl],
  ['download.changed', validateDownloadEvent],
  ['permission.changed', validatePermissionEvent],
  [
    'window.created',
    (value, path) =>
      value === undefined || value === null
        ? undefined
        : validateFrostWindow(value, path),
  ],
  [
    'window.closed',
    (value, path) =>
      value === undefined || value === null
        ? undefined
        : validateIdEvent(value, path, 'windowId'),
  ],
  [
    'window.focused',
    (value, path) =>
      value === undefined || value === null
        ? undefined
        : validateIdEvent(value, path, 'windowId'),
  ],
  [
    'external.audit',
    (value, path) => {
      const item = record(value, path);
      return {
        commandId: string(item.commandId, `${path}.commandId`, true),
        capability: validateExternalCapability(
          item.capability,
          `${path}.capability`,
        ),
        allowed: boolean(item.allowed, `${path}.allowed`),
        ...(item.reason === undefined
          ? {}
          : {
              reason:
                item.reason === null
                  ? null
                  : string(item.reason, `${path}.reason`),
            }),
      };
    },
  ],
  [
    'external.rateLimited',
    (value, path) => {
      const item = record(value, path);
      return {
        commandId: string(item.commandId, `${path}.commandId`, true),
        retryAfterMs: integer(item.retryAfterMs, `${path}.retryAfterMs`),
      };
    },
  ],
]);

for (const eventName of [
  'tabs.created',
  'tabs.updated',
  'tabs.closed',
  'tabs.activated',
  'downloads.updated',
  'host.synced',
  'app.stateChanged',
]) {
  eventValidators.set(eventName, validateVoid);
}

export function validateBridgeEvent<K extends keyof EventMap>(
  eventName: K,
  value: unknown,
): EventMap[K] {
  try {
    const validator = eventValidators.get(eventName);
    if (!validator) throw new Error('unsupported event name');
    return validator(value, 'payload') as EventMap[K];
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    throw new Error(`Invalid event "${eventName}": ${reason}`);
  }
}
