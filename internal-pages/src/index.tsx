import { createEffect, createResource, createSignal, Match, Show, Switch } from "solid-js";
import { render } from "solid-js/web";
import { loadPageData, resolvePage } from "./data";
import { Logo } from "./components";
import { japaneseLabels } from "./labels";
import { NewTab } from "./pages/NewTab";
import { Records } from "./pages/Records";
import { Settings } from "./pages/Settings";
import { Debug } from "./pages/Debug";
import "./styles.css";

function App() {
  const page = resolvePage(new URL(location.href));
  const [failed, setFailed] = createSignal(false);
  const [data, { refetch }] = createResource(async () => {
    setFailed(false);
    try {
      return await loadPageData();
    } catch {
      setFailed(true);
      return undefined;
    }
  });
  const t = (label: string) => (data()?.language === "ja" ? japaneseLabels[label] || label : label);
  const title = () =>
    t(
      {
        newtab: "New Tab",
        history: "History",
        bookmarks: "Bookmarks",
        downloads: "Downloads",
        settings: "Settings",
        debug: "Debug",
      }[page],
    );
  createEffect(() => {
    document.title = title();
    document.documentElement.lang = data()?.language || "en";
    document.documentElement.dataset.appearance = data()?.appearance || "system";
  });
  return (
    <Show
      when={!failed()}
      fallback={
        <main>
          <h1>{title()}</h1>
          <p class="error" role="alert">
            {t("Could not load this page. Please try again.")}
          </p>
          <button class="button" onClick={() => void refetch()}>
            {t("Reload")}
          </button>
        </main>
      }
    >
      <Show when={data()} fallback={<main aria-busy="true">{t("Loading…")}</main>}>
        {(loaded) => (
          <Show when={page !== "newtab"} fallback={<NewTab t={t} />}>
            <main>
              <header>
                <Logo />
                <h1>{title()}</h1>
              </header>
              <Switch>
                <Match when={page === "settings"}>
                  <Settings data={loaded()} t={t} />
                </Match>
                <Match when={page === "debug"}>
                  <Debug data={loaded()} t={t} />
                </Match>
                <Match when={["history", "bookmarks", "downloads"].includes(page)}>
                  <Records page={page} data={loaded()} t={t} />
                </Match>
              </Switch>
            </main>
          </Show>
        )}
      </Show>
    </Show>
  );
}
render(() => <App />, document.getElementById("root")!);
