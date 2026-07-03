import { createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import { commands, page, tabs, type BrowserCommand } from "../../bridge/fubuki";
import { t } from "../../i18n";
import { browserState, refreshState } from "../../stores/browserStore";
import { filterCommands, type PaletteCommand } from "./commands";

type Props = {
  open: boolean;
  quietMode: boolean;
  onClose: () => void;
  onToggleQuietMode: () => void;
};

const allowedCommandIds = new Set([
  "tabs.create",
  "tabs.close",
  "tabs.reopenClosed",
  "app.openSettings",
  "app.openHistory",
  "app.openBookmarks",
  "app.openDownloads",
  "app.toggleSidebar",
  "page.zoomReset",
  "app.openDebug",
  "app.openDevTools"
]);

function activeTabId() {
  return browserState.activeTabId;
}

function localizeCommand(command: BrowserCommand) {
  switch (command.id) {
    case "tabs.create":
      return t("common.newTab", browserState.settings.language);
    case "tabs.close":
      return t("action.closeTab", browserState.settings.language);
    case "tabs.reopenClosed":
      return browserState.settings.language === "ja" ? "閉じたタブを再度開く" : command.title;
    case "app.openSettings":
      return t("common.settings", browserState.settings.language);
    case "app.openHistory":
      return t("common.history", browserState.settings.language);
    case "app.openBookmarks":
      return t("common.bookmarks", browserState.settings.language);
    case "app.openDownloads":
      return t("common.downloads", browserState.settings.language);
    case "app.toggleSidebar":
      return t("common.toggleSidebar", browserState.settings.language);
    case "page.zoomReset":
      return browserState.settings.language === "ja" ? "表示倍率をリセット" : command.title;
    case "app.openDebug":
      return t("common.debug", browserState.settings.language);
    case "app.openDevTools":
      return browserState.settings.language === "ja" ? "DevToolsを開く" : command.title;
    default:
      return command.title;
  }
}

export default function CommandPalette(props: Props) {
  const [query, setQuery] = createSignal("");
  const [selectedIndex, setSelectedIndex] = createSignal(0);
  let inputRef: HTMLInputElement | undefined;

  const paletteCommands = createMemo<PaletteCommand[]>(() => {
    const native = browserState.commands
      .filter((command) => allowedCommandIds.has(command.id))
      .map((command) => ({
        ...command,
        title: localizeCommand(command),
        keywords: command.id.replaceAll(".", " "),
        run: async () => {
          if (command.id === "tabs.close") {
            const tabId = activeTabId();
            if (tabId) await tabs.close(tabId);
            return;
          }
          if (command.id === "page.zoomReset") {
            await page.zoomReset();
            return;
          }
          await commands.execute(command.id);
          await refreshState(command.id);
        }
      }));

    return [
      {
        id: "ui.toggleQuietMode",
        title: `${t("commandPalette.toggleQuietMode", browserState.settings.language)}${props.quietMode ? " ✓" : ""}`,
        category: "UI",
        shortcut: "",
        keywords: "quiet focus minimal zen",
        run: props.onToggleQuietMode
      },
      ...native
    ];
  });

  const filtered = createMemo(() => filterCommands(paletteCommands(), query()));

  createEffect(() => {
    if (props.open) {
      setQuery("");
      setSelectedIndex(0);
      queueMicrotask(() => inputRef?.focus());
    }
  });

  createEffect(() => {
    if (selectedIndex() >= filtered().length) {
      setSelectedIndex(Math.max(0, filtered().length - 1));
    }
  });

  const runSelected = async () => {
    const command = filtered()[selectedIndex()];
    if (!command) return;
    await command.run();
    props.onClose();
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (!props.open) return;
    if (event.key === "Escape") {
      event.preventDefault();
      props.onClose();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelectedIndex((index) => Math.min(index + 1, filtered().length - 1));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelectedIndex((index) => Math.max(index - 1, 0));
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      void runSelected();
    }
  };

  window.addEventListener("keydown", onKeyDown);
  onCleanup(() => window.removeEventListener("keydown", onKeyDown));

  return (
    <Show when={props.open}>
      <div class="command-palette-backdrop" onMouseDown={props.onClose}>
        <section
          class="command-palette"
          role="dialog"
          aria-modal="true"
          aria-label={t("commandPalette.title", browserState.settings.language)}
          onMouseDown={(event) => event.stopPropagation()}
        >
          <input
            ref={inputRef}
            class="command-palette-input"
            value={query()}
            placeholder={t("commandPalette.placeholder", browserState.settings.language)}
            aria-label={t("commandPalette.placeholder", browserState.settings.language)}
            autocomplete="off"
            onInput={(event) => {
              setQuery(event.currentTarget.value);
              setSelectedIndex(0);
            }}
          />
          <div class="command-palette-list" role="listbox">
            <Show when={filtered().length > 0} fallback={<p class="command-palette-empty">{t("commandPalette.empty", browserState.settings.language)}</p>}>
              <For each={filtered()}>
                {(command, index) => (
                  <button
                    classList={{ "command-palette-row": true, selected: index() === selectedIndex() }}
                    role="option"
                    aria-selected={index() === selectedIndex()}
                    onMouseEnter={() => setSelectedIndex(index())}
                    onClick={() => void runSelected()}
                  >
                    <span>
                      <span class="command-palette-title">{command.title}</span>
                      <span class="command-palette-category">{command.category}</span>
                    </span>
                    <Show when={command.shortcut}>
                      <kbd>{command.shortcut}</kbd>
                    </Show>
                  </button>
                )}
              </For>
            </Show>
          </div>
        </section>
      </div>
    </Show>
  );
}
