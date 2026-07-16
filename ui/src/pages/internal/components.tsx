import { fubukiLogoDataUri } from '../../assets/logo';

export function Logo() {
  return <img class="internal-logo" src={fubukiLogoDataUri} alt="" />;
}

export function PageHeader(props: { title: string }) {
  return <header class="internal-header"><Logo /><h1>{props.title}</h1></header>;
}

export function EmptyState(props: { children: string }) {
  return <p class="internal-empty">{props.children}</p>;
}

export function ActionButton(props: {
  keyName: string; value: string; returnUrl: string; children: string;
  danger?: boolean; selected?: boolean;
}) {
  const params = new URLSearchParams({ key: props.keyName, value: props.value, return: props.returnUrl });
  const method = props.danger ? 'post' : 'get';
  return <form method={method} action={`fubuki://settings/set?${params}`}><input type="hidden" name="key" value={props.keyName} /><input type="hidden" name="value" value={props.value} /><input type="hidden" name="return" value={props.returnUrl} /><button class={`internal-chip${props.danger ? ' danger' : ''}${props.selected ? ' selected' : ''}`}>{props.children}</button></form>;
}

export function SettingChoice(props: { keyName: string; value: string; label: string; selected?: boolean }) {
  return <ActionButton keyName={props.keyName} value={props.value} returnUrl="fubuki://settings/" selected={props.selected}>{props.label}</ActionButton>;
}

export function SettingInput(props: { keyName: string; placeholder: string }) {
  return <form class="inline-form" action="fubuki://settings/set" method="get"><input type="hidden" name="key" value={props.keyName} /><input type="hidden" name="return" value="fubuki://settings/" /><input name="value" placeholder={props.placeholder} /><button class="internal-button">Save</button></form>;
}
