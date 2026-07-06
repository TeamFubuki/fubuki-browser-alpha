import { describe, expect, it } from "vitest";
import { commandMatches, filterCommands } from "../components/commandPalette/commands";

const commands = [
  {
    id: "tabs.create",
    title: "New Tab",
    category: "Tabs",
    shortcut: "Cmd+T",
    keywords: "tabs.create",
    run: () => {},
  },
  {
    id: "tabs.close",
    title: "Close Tab",
    category: "Tabs",
    shortcut: "Cmd+W",
    keywords: "tabs.close",
    run: () => {},
  },
  {
    id: "app.openSettings",
    title: "Settings",
    category: "App",
    shortcut: "Cmd+,",
    keywords: "app.openSettings preferences",
    run: () => {},
  },
  {
    id: "page.zoomReset",
    title: "Reset Zoom",
    category: "Page",
    shortcut: "Cmd+0",
    keywords: "page.zoomReset zoom",
    run: () => {},
  },
];

describe("commandMatches", () => {
  it("matches by id", () => {
    expect(commandMatches(commands[0], "tabs.create")).toBe(true);
  });

  it("matches by title (case-insensitive)", () => {
    expect(commandMatches(commands[0], "new tab")).toBe(true);
    expect(commandMatches(commands[0], "NEW TAB")).toBe(true);
  });

  it("matches by category", () => {
    expect(commandMatches(commands[0], "tabs")).toBe(true);
  });

  it("matches by keywords", () => {
    expect(commandMatches(commands[2], "preferences")).toBe(true);
  });

  it("matches by shortcut", () => {
    expect(commandMatches(commands[2], "cmd+")).toBe(true);
  });

  it("returns true for empty query", () => {
    expect(commandMatches(commands[0], "")).toBe(true);
    expect(commandMatches(commands[0], "   ")).toBe(true);
  });

  it("returns false for non-matching query", () => {
    expect(commandMatches(commands[0], "xyz")).toBe(false);
  });

  it("matches partial strings", () => {
    expect(commandMatches(commands[0], "creat")).toBe(true);
    expect(commandMatches(commands[3], "zoom")).toBe(true);
  });
});

describe("filterCommands", () => {
  it("returns all commands for empty query", () => {
    expect(filterCommands(commands, "")).toHaveLength(4);
  });

  it("filters by id", () => {
    const result = filterCommands(commands, "tabs.create");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("tabs.create");
  });

  it("filters by title", () => {
    const result = filterCommands(commands, "settings");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("app.openSettings");
  });

  it("filters by category", () => {
    const result = filterCommands(commands, "App");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("app.openSettings");
  });

  it("filters by keywords", () => {
    const result = filterCommands(commands, "preferences");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("app.openSettings");
  });

  it("returns empty array for no matches", () => {
    const result = filterCommands(commands, "nonexistent");
    expect(result).toHaveLength(0);
  });

  it("matches multiple commands with same prefix", () => {
    const result = filterCommands(commands, "tabs");
    expect(result).toHaveLength(2);
    expect(result.map((c) => c.id)).toEqual(["tabs.create", "tabs.close"]);
  });
});
