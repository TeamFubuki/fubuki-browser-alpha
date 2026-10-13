import { createSignal, For, Show } from "solid-js";
import { actionUrl, formatTime, type PageData } from "../data";
import { Action, Field, type Translate } from "../components";

const sections = [
  {
    id: "general",
    label: "General",
    key: "startupBehavior",
    fallback: "newTab",
    choices: [
      ["newTab", "New tab"],
      ["restore", "Restore previous session"],
      ["homePage", "Home page"],
    ],
    inputs: [["homeUrl", "Home page URL", "https://example.com"]],
  },
  {
    id: "appearance",
    label: "Appearance",
    key: "appearance",
    fallback: "system",
    choices: [
      ["system", "System"],
      ["light", "Light"],
      ["dark", "Dark"],
    ],
    inputs: [],
  },
  {
    id: "language",
    label: "Language",
    key: "language",
    fallback: "system",
    choices: [
      ["system", "System"],
      ["ja", "Japanese"],
      ["en", "English"],
    ],
    inputs: [],
  },
  {
    id: "tabs",
    label: "Tabs",
    key: "newTabPage",
    fallback: "blank",
    choices: [
      ["blank", "Blank new tab"],
      ["home", "Home on new tab"],
    ],
    inputs: [["defaultZoomLevel", "Default zoom level", "0"]],
  },
  {
    id: "windows",
    label: "Windows",
    key: "sidebarVisible",
    fallback: "show",
    choices: [
      ["show", "Show sidebar"],
      ["hide", "Hide sidebar"],
    ],
    inputs: [["sidebarWidth", "Sidebar width", "196"]],
  },
  {
    id: "search",
    label: "Search",
    key: "searchEngine",
    fallback: "google",
    choices: [
      ["google", "Google"],
      ["duckduckgo", "DuckDuckGo"],
      ["bing", "Bing"],
      ["custom", "Custom"],
    ],
    inputs: [["customSearchUrl", "Custom search URL", "https://www.google.com/search?q={query}"]],
  },
  {
    id: "downloads",
    label: "Downloads section",
    key: "askBeforeDownload",
    fallback: "off",
    choices: [
      ["on", "Ask before download"],
      ["off", "Download automatically"],
    ],
    inputs: [["downloadDirectory", "Download directory", ""]],
  },
];
function SettingInput(props: {
  data: PageData;
  settingKey: string;
  label: string;
  fallback: string;
  t: Translate;
}) {
  const [value, setValue] = createSignal(props.data.settings[props.settingKey] || props.fallback);
  return (
    <div>
      <form
        class="inline-form"
        method="post"
        action={actionUrl({ key: props.settingKey, value: value() }, "settings")}
      >
        <input type="hidden" name="key" value={props.settingKey} />
        <input type="hidden" name="return" value="fubuki://settings/" />
        <label>
          {props.t(props.label)}{" "}
          <input
            name="value"
            value={value()}
            onInput={(event) => setValue(event.currentTarget.value)}
          />
        </label>
        <button class="button">{props.t("Save")}</button>
      </form>
      <Action
        page="settings"
        params={{ key: "resetSetting", value: props.settingKey }}
        label={props.t("Reset")}
      />
    </div>
  );
}
export function Settings(props: { data: PageData; t: Translate }) {
  const [search, setSearch] = createSignal("");
  const matches = (text: string) => text.toLowerCase().includes(search().toLowerCase());
  const nav = () => [
    ...sections.map(({ id, label }) => ({ id, label })),
    { id: "privacy", label: "Privacy" },
    { id: "permissions", label: "Permissions" },
    { id: "developer", label: "Developer" },
  ];
  return (
    <div class="settings-layout">
      <nav class="settings-nav" aria-label={props.t("Settings sections")}>
        <For each={nav()}>
          {(section) => <a href={`#${section.id}`}>{props.t(section.label)}</a>}
        </For>
      </nav>
      <div class="settings-content">
        <input
          type="search"
          aria-label={props.t("Search settings")}
          placeholder={props.t("Search settings")}
          value={search()}
          onInput={(event) => setSearch(event.currentTarget.value)}
        />
        <For each={sections}>
          {(section) => (
            <Show
              when={matches(
                [
                  props.t(section.label),
                  ...section.choices.map(([, label]) => props.t(label)),
                  ...section.inputs.map(([, label]) => props.t(label)),
                ].join(" "),
              )}
            >
              <Field id={section.id} label={props.t(section.label)}>
                <div class="segmented">
                  <For each={section.choices}>
                    {([value, label]) => (
                      <Action
                        page="settings"
                        params={{ key: section.key, value }}
                        label={props.t(label)}
                        selected={(props.data.settings[section.key] || section.fallback) === value}
                      />
                    )}
                  </For>
                </div>
                <For each={section.inputs}>
                  {([key, label, fallback]) => (
                    <SettingInput
                      data={props.data}
                      settingKey={key}
                      label={label}
                      fallback={fallback}
                      t={props.t}
                    />
                  )}
                </For>
                <Action
                  page="settings"
                  params={{ key: "resetSetting", value: section.key }}
                  label={props.t("Reset")}
                />
              </Field>
            </Show>
          )}
        </For>
        <Show when={matches(`${props.t("Privacy")} ${props.t("Clear all")}`)}>
          <Field id="privacy" label={props.t("Privacy")}>
            <div class="segmented">
              <For
                each={[
                  ["history", "History"],
                  ["cookies", "Cookies"],
                  ["cache", "Cache"],
                  ["downloads", "Downloads"],
                  ["all", "Clear all"],
                ]}
              >
                {([value, label]) => (
                  <Action
                    page="settings"
                    params={{ key: "clearData", value }}
                    label={props.t(label)}
                    danger
                  />
                )}
              </For>
            </div>
          </Field>
        </Show>
        <Show
          when={matches(
            `${props.t("Permissions")} ${props.data.permissions.map((permission) => permission.origin).join(" ")}`,
          )}
        >
          <Field id="permissions" label={props.t("Permission decisions")}>
            <Show
              when={props.data.permissions.length}
              fallback={<p class="empty">{props.t("No saved permission decisions.")}</p>}
            >
              <div class="list">
                <For each={props.data.permissions}>
                  {(permission) => (
                    <article class="field">
                      <strong>{permission.origin}</strong>
                      <span class="meta">
                        {props.t(
                          (
                            {
                              camera: "Camera",
                              microphone: "Microphone",
                              geolocation: "Location",
                              notifications: "Notifications",
                              pointerLock: "Pointer lock",
                              keyboardLock: "Keyboard lock",
                            } as Record<string, string>
                          )[permission.permission] || permission.permission,
                        )}{" "}
                        · {props.t(permission.value === "allow" ? "Allow" : "Block")} ·{" "}
                        {formatTime(permission.createdAt)}
                      </span>
                      <div class="segmented">
                        <For
                          each={[
                            ["allow", "Allow"],
                            ["block", "Block"],
                            ["ask", "Reset"],
                          ]}
                        >
                          {([value, label]) => (
                            <Action
                              page="settings"
                              params={{
                                key: value === "ask" ? "removePermission" : "setPermission",
                                origin: permission.origin,
                                permission: permission.permission,
                                value,
                              }}
                              label={props.t(label)}
                              selected={
                                value !== "ask" &&
                                (permission.value === value ||
                                  (value === "block" && permission.value === "deny"))
                              }
                              danger={value === "ask"}
                            />
                          )}
                        </For>
                      </div>
                    </article>
                  )}
                </For>
              </div>
            </Show>
          </Field>
        </Show>
        <Show when={matches("Shortcuts Cmd")}>
          <Field label={props.t("Shortcuts")}>
            <p class="meta">
              Cmd+T, Cmd+N, Cmd+Shift+N, Cmd+W, Cmd+Shift+T, Cmd+L, Cmd+R, Cmd+F, Cmd+,, Cmd+Plus,
              Cmd+Minus, Cmd+0
            </p>
          </Field>
        </Show>
        <Show when={matches(props.t("Developer"))}>
          <Field id="developer" label={props.t("Developer")}>
            <a class="chip" href="fubuki://debug/">
              {props.t("Debug page")}
            </a>
          </Field>
        </Show>
      </div>
    </div>
  );
}
