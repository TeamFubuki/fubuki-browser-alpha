import { Match, Switch, createEffect, createSignal, onMount } from 'solid-js';
import SettingsSidebar from '../components/settings/SettingsSidebar';
import { settingsSections, type SettingsSection } from '../components/settings/settingsSections';
import AppearancePanel from '../components/settings/settings-panels/AppearancePanel';
import GeneralPanel from '../components/settings/settings-panels/GeneralPanel';
import SearchPanel from '../components/settings/settings-panels/SearchPanel';
import ZoomPanel from '../components/settings/settings-panels/ZoomPanel';
import AboutPanel from '../components/settings/settings-panels/AboutPanel';
import McpPanel from '../components/settings/settings-panels/McpPanel';
import { t } from '../i18n';
import { browserState, activeTab, navigateInternal } from '../stores/browserStore';

// Persist scroll position across re-renders
let savedScrollPosition = 0;

// Extract section from URL like fubuki://settings/general
function getSectionFromUrl(): SettingsSection | null {
  const tab = activeTab();
  if (!tab?.url) return null;
  const url = tab.url;
  
  const settingsMatch = url.match(/^fubuki:\/\/settings\/([a-z]+)\/?$/);
  if (settingsMatch) {
    const section = settingsMatch[1] as SettingsSection;
    if (settingsSections.some((s) => s.id === section)) {
      return section;
    }
  }
  
  return null;
}

export default function SettingsPage() {
  // Initialize section from URL if available
  const initialSection = getSectionFromUrl();
  const [activeSection, setActiveSection] = createSignal<SettingsSection>(initialSection ?? 'general');
  let scrollContainerRef: HTMLDivElement | undefined;
  const lang = () => browserState.settings.language;
  const activeDefinition = () =>
    settingsSections.find((section) => section.id === activeSection()) ??
    settingsSections[0];

  // Update section when URL changes
  createEffect(() => {
    const section = getSectionFromUrl();
    if (section) {
      setActiveSection(section);
    }
  });

  // Restore scroll position after re-render
  onMount(() => {
    if (scrollContainerRef && savedScrollPosition > 0) {
      requestAnimationFrame(() => {
        if (scrollContainerRef) {
          scrollContainerRef.scrollTop = savedScrollPosition;
        }
      });
    }
  });

  // Save scroll position on scroll
  const handleScroll = () => {
    if (scrollContainerRef) {
      savedScrollPosition = scrollContainerRef.scrollTop;
    }
  };

  const selectSection = (section: SettingsSection) => {
    setActiveSection(section);
    savedScrollPosition = 0;
    const url =
      section === 'general'
        ? 'fubuki://settings/'
        : `fubuki://settings/${section}`;
    if (activeTab()?.url !== url) {
      navigateInternal(url);
    }
  };

  return (
    <section class="settings-page" aria-label={t('common.settings', lang())}>
      <SettingsSidebar
        activeSection={activeSection()}
        onSelect={selectSection}
      />
      <main class="settings-content">
        <div class="settings-mobile-switcher">
          <label for="settings-section-select">{t('common.settings', lang())}</label>
          <select
            id="settings-section-select"
            value={activeSection()}
            onChange={(event) =>
              selectSection(event.currentTarget.value as SettingsSection)
            }
          >
            {settingsSections.map((section) => (
              <option value={section.id}>{t(section.titleKey, lang())}</option>
            ))}
          </select>
        </div>

        <header class="settings-content-header">
          <div class="settings-content-icon" aria-hidden="true">
            {activeDefinition().icon}
          </div>
          <div>
            <h1>{t(activeDefinition().titleKey, lang())}</h1>
            <p>{t(activeDefinition().descriptionKey, lang())}</p>
          </div>
        </header>

        <div
          ref={scrollContainerRef}
          class="settings-scroll"
          onScroll={handleScroll}
        >
          <Switch>
            <Match when={activeSection() === 'general'}>
              <GeneralPanel />
            </Match>
            <Match when={activeSection() === 'appearance'}>
              <AppearancePanel />
            </Match>
            <Match when={activeSection() === 'search'}>
              <SearchPanel />
            </Match>
            <Match when={activeSection() === 'zoom'}>
              <ZoomPanel />
            </Match>
            <Match when={activeSection() === 'mcp'}>
              <McpPanel />
            </Match>
            <Match when={activeSection() === 'about'}>
              <AboutPanel />
            </Match>
          </Switch>
        </div>
      </main>
    </section>
  );
}
