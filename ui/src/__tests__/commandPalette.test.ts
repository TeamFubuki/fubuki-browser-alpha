import { describe, expect, it, vi } from 'vitest';
import {
  filterCommands,
  type PaletteCommand,
} from '../components/commandPalette/commands';

const commands: PaletteCommand[] = [
  {
    id: 'tabs.create',
    title: '新しいタブ',
    category: 'Tabs',
    shortcut: 'Cmd+T',
    run: vi.fn(),
  },
  {
    id: 'app.openSettings',
    title: '設定',
    category: 'App',
    shortcut: 'Cmd+,',
    keywords: 'preferences',
    run: vi.fn(),
  },
];

describe('command palette filtering', () => {
  it('matches localized titles and ids', () => {
    expect(
      filterCommands(commands, '設定').map((command) => command.id),
    ).toEqual(['app.openSettings']);
    expect(
      filterCommands(commands, 'tabs').map((command) => command.id),
    ).toEqual(['tabs.create']);
  });

  it('matches keywords', () => {
    expect(
      filterCommands(commands, 'preferences').map((command) => command.id),
    ).toEqual(['app.openSettings']);
  });
});
