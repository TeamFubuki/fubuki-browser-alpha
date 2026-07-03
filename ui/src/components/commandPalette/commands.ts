import type { BrowserCommand } from "../../bridge/fubuki";
import type { I18nKey } from "../../i18n";

export type PaletteCommand = BrowserCommand & {
  keywords?: string;
  localTitleKey?: I18nKey;
  run: () => void | Promise<void>;
};

export function commandMatches(command: PaletteCommand, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return `${command.id} ${command.title} ${command.category} ${command.shortcut} ${command.keywords ?? ""}`
    .toLowerCase()
    .includes(q);
}

export function filterCommands(commands: PaletteCommand[], query: string): PaletteCommand[] {
  return commands.filter((command) => commandMatches(command, query));
}
