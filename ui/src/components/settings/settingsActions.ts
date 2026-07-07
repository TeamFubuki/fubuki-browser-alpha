import { invokeBridge } from '../../bridge/fubuki';

export async function setSetting(key: string, value: string): Promise<void> {
  await invokeBridge('settings.set', { key, value });
}

export async function resetSettings(keys: string[]): Promise<void> {
  await Promise.all(keys.map((key) => invokeBridge('settings.reset', { key })));
}

export function enabledValue(value: boolean): 'on' | 'off' {
  return value ? 'on' : 'off';
}
