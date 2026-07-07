import { For, createMemo } from 'solid-js';
import { t } from '../../i18n';
import { browserState } from '../../stores/browserStore';
import { invokeBridge } from '../../bridge/fubuki';
import SettingsDetail from './SettingsDetail';

interface Props {
  onBack: () => void;
}

const exposedTools = [
  'browser.snapshot',
  'tabs.list',
  'tabs.create',
  'tabs.navigate',
  'tabs.activate',
  'tabs.close',
  'page.getText',
  'page.getHtml',
  'page.click',
  'page.type',
  'bookmarks.list',
  'history.list',
  'downloads.list',
];

function enabledValue(value: boolean): 'on' | 'off' {
  return value ? 'on' : 'off';
}

export default function McpSettings(props: Props) {
  const lang = () => browserState.settings.language;
  const enabled = () => browserState.settings['automation.mcp.enabled'] === 'on';
  const confirmSensitive = () =>
    browserState.settings['automation.mcp.confirmSensitive'] !== 'off';
  const serverCommand = () =>
    browserState.settings['automation.mcp.serverCommand'] ||
    'target/debug/fubuki-mcp-server';
  const serverArgs = () => browserState.settings['automation.mcp.serverArgs'];
  const clientName = () =>
    browserState.settings['automation.mcp.clientName'] || 'fubuki';
  const configJson = createMemo(() =>
    JSON.stringify(
      {
        mcpServers: {
          [clientName() || 'fubuki']: {
            command: serverCommand(),
            args: serverArgs()
              .split(' ')
              .map((item) => item.trim())
              .filter(Boolean),
          },
        },
      },
      null,
      2,
    ),
  );

  const updateEnabled = async (checked: boolean) => {
    await invokeBridge('settings.set', {
      key: 'automation.mcp.enabled',
      value: enabledValue(checked),
    });
  };

  const updateConfirmSensitive = async (checked: boolean) => {
    await invokeBridge('settings.set', {
      key: 'automation.mcp.confirmSensitive',
      value: enabledValue(checked),
    });
  };

  const updateSetting = async (key: string, value: string) => {
    await invokeBridge('settings.set', { key, value });
  };

  return (
    <SettingsDetail
      icon="◇"
      titleKey="settings.mcp"
      descriptionKey="settings.mcpDescription"
      onBack={props.onBack}
    >
      <div class="settings-card">
        <div class="settings-card-row">
          <div>
            <div class="settings-card-label">
              {t('settings.mcpEnabled', lang())}
            </div>
            <div class="settings-card-description">
              {lang() === 'ja'
                ? 'AI機能を利用するための連携を有効にします'
                : 'Enable AI-powered features via MCP'}
            </div>
          </div>
          <label class="toggle-switch">
            <input
              type="checkbox"
              checked={enabled()}
              onChange={(e) => void updateEnabled(e.currentTarget.checked)}
            />
            <span class="toggle-track" />
            <span class="toggle-thumb" />
          </label>
        </div>
      </div>

      <div class="settings-card">
        <div class="settings-card-row">
          <div>
            <div class="settings-card-label">
              {t('settings.mcpConfirmSensitive', lang())}
            </div>
            <div class="settings-card-description">
              {t('settings.mcpConfirmSensitiveDescription', lang())}
            </div>
          </div>
          <label class="toggle-switch">
            <input
              type="checkbox"
              checked={confirmSensitive()}
              disabled={!enabled()}
              onChange={(e) =>
                void updateConfirmSensitive(e.currentTarget.checked)
              }
            />
            <span class="toggle-track" />
            <span class="toggle-thumb" />
          </label>
        </div>
      </div>

      <div class="settings-mcp-card">
        <div class="settings-mcp-header">
          <span class="settings-mcp-title">{t('settings.mcpServer', lang())}</span>
          <span class="settings-mcp-badge">
            {enabled() ? t('settings.mcpRunning', lang()) : t('settings.mcpStopped', lang())}
          </span>
        </div>

        <div classList={{ 'settings-mcp-fields': true, enabled: enabled() }}>
          <div class="setting-row">
            <label class="setting-label" for="setting-mcp-command">
              {t('settings.mcpServerCommand', lang())}
            </label>
            <input
              id="setting-mcp-command"
              class="setting-input"
              type="text"
              value={serverCommand()}
              placeholder="target/debug/fubuki-mcp-server"
              disabled={!enabled()}
              onBlur={(e) =>
                void updateSetting(
                  'automation.mcp.serverCommand',
                  e.currentTarget.value,
                )
              }
              onKeyDown={(e) => {
                if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
              }}
            />
            <p class="setting-description">
              {t('settings.mcpServerCommandDescription', lang())}
            </p>
          </div>

          <div class="setting-row">
            <label class="setting-label" for="setting-mcp-args">
              {t('settings.mcpServerArgs', lang())}
            </label>
            <input
              id="setting-mcp-args"
              class="setting-input"
              type="text"
              value={serverArgs()}
              placeholder=""
              disabled={!enabled()}
              onBlur={(e) =>
                void updateSetting(
                  'automation.mcp.serverArgs',
                  e.currentTarget.value,
                )
              }
              onKeyDown={(e) => {
                if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
              }}
            />
          </div>

          <div class="setting-row" style={{ 'margin-bottom': '0' }}>
            <label class="setting-label" for="setting-mcp-client-name">
              {t('settings.mcpClientName', lang())}
            </label>
            <input
              id="setting-mcp-client-name"
              class="setting-input"
              type="text"
              value={clientName()}
              placeholder="fubuki"
              disabled={!enabled()}
              onBlur={(e) =>
                void updateSetting(
                  'automation.mcp.clientName',
                  e.currentTarget.value,
                )
              }
              onKeyDown={(e) => {
                if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
              }}
            />
          </div>
        </div>
      </div>

      <div class="settings-card">
        <div class="setting-row" style={{ 'margin-bottom': '0' }}>
          <div class="setting-label">{t('settings.mcpClientConfig', lang())}</div>
          <pre class="settings-code-block">{configJson()}</pre>
        </div>
      </div>

      <div class="settings-card">
        <div class="setting-row" style={{ 'margin-bottom': '0' }}>
          <div class="setting-label">{t('settings.mcpTools', lang())}</div>
          <div class="settings-chip-list">
            <For each={exposedTools}>
              {(tool) => <span class="settings-chip">{tool}</span>}
            </For>
          </div>
        </div>
      </div>
    </SettingsDetail>
  );
}
