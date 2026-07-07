import { For, createSignal } from 'solid-js';
import { t } from '../../../i18n';
import { browserState } from '../../../stores/browserStore';
import {
  ActionButton,
  SettingRow,
  SettingsGroup,
  TextInput,
  Toggle,
} from '../settings-ui';
import { setSetting } from '../settingsActions';

const exposedTools = [
  'browser.snapshot',
  'tabs.list',
  'tabs.create',
  'tabs.navigate',
  'tabs.activate',
  'tabs.close',
  'tabs.reload',
  'tabs.goBack',
  'tabs.goForward',
  'page.getText',
  'page.getHtml',
  'page.click',
  'page.type',
  'page.press',
  'page.scroll',
  'page.find',
  'bookmarks.list',
  'history.list',
  'downloads.list',
];

function enabledValue(value: boolean): 'on' | 'off' {
  return value ? 'on' : 'off';
}

export default function McpPanel() {
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
  const [copied, setCopied] = createSignal(false);

  const configJson = () =>
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
    );

  const copyConfig = async () => {
    try {
      await navigator.clipboard.writeText(configJson());
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      setCopied(false);
    }
  };

  return (
    <div class="settings-panel settings-panel-wide">
      <SettingsGroup
        title={t('mcp.page.status', lang())}
        description={
          enabled()
            ? t('settings.mcpConnectionReady', lang())
            : t('settings.mcpConnectionDisabled', lang())
        }
      >
        <SettingRow
          label={t('settings.mcpEnabled', lang())}
          description={t('settings.mcpDescription', lang())}
          control={
            <Toggle
              label={t('settings.mcpEnabled', lang())}
              checked={enabled()}
              onChange={(checked) =>
                void setSetting('automation.mcp.enabled', enabledValue(checked))
              }
            />
          }
        />
        <SettingRow
          label={t('settings.mcpConfirmSensitive', lang())}
          description={t('settings.mcpConfirmSensitiveDescription', lang())}
          control={
            <Toggle
              label={t('settings.mcpConfirmSensitive', lang())}
              checked={confirmSensitive()}
              disabled={!enabled()}
              onChange={(checked) =>
                void setSetting(
                  'automation.mcp.confirmSensitive',
                  enabledValue(checked),
                )
              }
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup
        title={t('settings.mcpServer', lang())}
        description={t('settings.mcpServerCommandDescription', lang())}
      >
        <SettingRow
          label={t('settings.mcpServerCommand', lang())}
          control={
            <TextInput
              value={serverCommand()}
              placeholder="target/debug/fubuki-mcp-server"
              disabled={!enabled()}
              onCommit={(value) =>
                void setSetting('automation.mcp.serverCommand', value)
              }
            />
          }
        />
        <SettingRow
          label={t('settings.mcpServerArgs', lang())}
          control={
            <TextInput
              value={serverArgs()}
              disabled={!enabled()}
              onCommit={(value) =>
                void setSetting('automation.mcp.serverArgs', value)
              }
            />
          }
        />
        <SettingRow
          label={t('settings.mcpClientName', lang())}
          control={
            <TextInput
              value={clientName()}
              placeholder="fubuki"
              disabled={!enabled()}
              onCommit={(value) =>
                void setSetting('automation.mcp.clientName', value)
              }
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup
        title={t('settings.mcpClientConfig', lang())}
        description={t('mcp.page.copyConfigDescription', lang())}
        actions={
          <ActionButton variant="primary" onClick={() => void copyConfig()}>
            {copied()
              ? lang() === 'ja'
                ? 'コピー済み'
                : 'Copied'
              : t('settings.mcpCopyConfig', lang())}
          </ActionButton>
        }
      >
        <pre class="settings-code-block">{configJson()}</pre>
      </SettingsGroup>

      <SettingsGroup
        title={t('settings.mcpTools', lang())}
        description={t('mcp.page.toolsDescription', lang())}
      >
        <div class="settings-chip-list">
          <For each={exposedTools}>
            {(tool) => <span class="settings-chip">{tool}</span>}
          </For>
        </div>
      </SettingsGroup>

      <SettingsGroup title={t('mcp.page.setupInstructions', lang())}>
        <div class="settings-step-list">
          <div class="settings-step">
            <span>1</span>
            <div>
              <strong>{t('mcp.page.buildServer', lang())}</strong>
              <pre class="settings-code-block">cargo build -p fubuki-mcp-server</pre>
            </div>
          </div>
          <div class="settings-step">
            <span>2</span>
            <div>
              <strong>{t('mcp.page.copyConfig', lang())}</strong>
              <p>{t('mcp.page.copyConfigDescription', lang())}</p>
            </div>
          </div>
        </div>
      </SettingsGroup>
    </div>
  );
}
