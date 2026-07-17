type NativeQuery = {
  request: string;
  onSuccess: (response: string) => void;
  onFailure: (code: number, message: string) => void;
};

declare global {
  interface Window {
    cefQuery?: (query: NativeQuery) => void;
    fubukiInternalMarker?: boolean;
  }
}

export type InternalActionFeedback = {
  kind: 'success' | 'error';
  message: string;
};

export const INTERNAL_ACTION_FEEDBACK_EVENT = 'fubuki:internal-action-feedback';
export const INTERNAL_DATA_CHANGED_EVENT = 'fubuki:internal-data-changed';

let interactionScrollPosition: { x: number; y: number } | undefined;

export function rememberInternalScrollPosition() {
  interactionScrollPosition = { x: window.scrollX, y: window.scrollY };
}

function changedEvent() {
  const detail = interactionScrollPosition ?? {
    x: window.scrollX,
    y: window.scrollY,
  };
  interactionScrollPosition = undefined;
  return new CustomEvent(INTERNAL_DATA_CHANGED_EVENT, { detail });
}

export function announceInternalAction(feedback: InternalActionFeedback) {
  window.dispatchEvent(
    new CustomEvent<InternalActionFeedback>(INTERNAL_ACTION_FEEDBACK_EVENT, {
      detail: feedback,
    }),
  );
}

/**
 * Sends a mutation over the capability-limited internal-page channel.
 * This is intentionally separate from the privileged Frost Protocol bridge.
 */
export function invokeInternalAction(
  key: string,
  value: string,
): Promise<void> {
  if (import.meta.env.DEV && !window.fubukiInternalMarker) {
    window.dispatchEvent(changedEvent());
    return Promise.resolve();
  }
  if (!window.fubukiInternalMarker || !window.cefQuery) {
    return Promise.reject(
      new Error('The internal page action channel is not available.'),
    );
  }

  return new Promise<void>((resolve, reject) => {
    window.cefQuery?.({
      request: JSON.stringify({
        channel: 'internal.action',
        key,
        value,
      }),
      onSuccess: (response) => {
        try {
          const result = JSON.parse(response) as { ok?: boolean };
          if (result.ok) {
            window.dispatchEvent(changedEvent());
            resolve();
            return;
          }
        } catch {
          // Fall through to the consistent user-facing error below.
        }
        reject(new Error('The action could not be completed.'));
      },
      onFailure: (_code, message) =>
        reject(new Error(message || 'The action could not be completed.')),
    });
  });
}
