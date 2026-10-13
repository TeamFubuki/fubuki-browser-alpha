import { For } from "solid-js";
import type { PageData } from "../data";
import { Action, Field, type Translate } from "../components";

export function Debug(props: { data: PageData; t: Translate }) {
  return (
    <div class="section">
      <Field label="Bridge">
        <span class="meta">Frost Protocol v0 · app origin only</span>
      </Field>
      <Field label={props.t("Profile path")}>
        <span class="meta">{props.data.profilePath}</span>
      </Field>
      <Field label={props.t("Windows and tabs")}>
        <For each={props.data.windows}>
          {(window) => (
            <article class="field">
              <strong>
                {window.id}
                {window.isPrivate ? " (Private)" : ""}
              </strong>
              <span class="meta">
                {window.tabs.map((tab) => `${tab.isActive ? "* " : ""}${tab.title}`).join(" · ")}
              </span>
            </article>
          )}
        </For>
      </Field>
      <Field label={props.t("Registered commands")}>
        <For each={props.data.commands}>
          {(command) => (
            <article class="field">
              <strong>{command.title}</strong>
              <span class="meta">
                {command.id} · {command.shortcut}
              </span>
            </article>
          )}
        </For>
      </Field>
      <Field label={props.t("Recent events")}>
        <For each={props.data.events}>
          {(event) => (
            <article class="field">
              <strong>{event.name}</strong>
              <span class="meta">{event.message}</span>
            </article>
          )}
        </For>
      </Field>
      <Field label={props.t("Logs")}>
        <For each={props.data.records}>
          {(record) => (
            <article class="field">
              <strong>{record.title}</strong>
              <span class="meta">
                {record.createdAt} · {record.path}
              </span>
            </article>
          )}
        </For>
      </Field>
      <Field label={props.t("Actions")}>
        <Action
          page="debug"
          params={{ key: "openDevTools", value: "1" }}
          label={props.t("Open DevTools")}
        />
      </Field>
    </div>
  );
}
