import { onBeforeUnmount, onMounted, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { errorCode, getHistory, getState } from '@/services/client';
import { setLocale } from '@/i18n/i18n';
import { setTheme } from '@/composables/useTheme';
import type { ClientError, HistoryEntry, Snapshot } from '@/types';

export const STATE_EVENT = 'client-state';

export const REFRESH_MS = 30_000;

/**
 * Window state refreshed by native events and a visible-window polling fallback.
 * History is requested only after a NAS connection is available.
 * @param historyLimit - How many entries of the history the window shows.
 */
export function useClient(historyLimit: number) {
  const snapshot = ref<Snapshot | null>(null);
  const history = ref<HistoryEntry[]>([]);
  const error = ref<ClientError | null>(null);
  const busy = ref(false);
  let timer: ReturnType<typeof setInterval> | undefined;
  let unlisten: UnlistenFn | undefined;

  async function refresh() {
    try {
      const state = await getState();
      const entries = state.status === 'unconfigured' ? [] : await getHistory(historyLimit);
      snapshot.value = state;
      history.value = entries;
      setLocale(state.appearance.locale);
      setTheme(state.appearance.theme);
    } catch (reason) {
      error.value = errorCode(reason);
    }
  }

  /**
   * Runs a change of the client, one at a time, then reads the state again.
   * @param action - The change to run.
   * @returns Whether it succeeded.
   */
  async function run(action: () => Promise<unknown>) {
    if (busy.value) {
      return false;
    }
    busy.value = true;
    error.value = null;
    try {
      await action();
      return true;
    } catch (reason) {
      error.value = errorCode(reason);
      return false;
    } finally {
      busy.value = false;
      await refresh();
    }
  }

  function refreshVisible() {
    if (!document.hidden) {
      void refresh();
    }
  }

  onMounted(async () => {
    void refresh();
    timer = setInterval(refreshVisible, REFRESH_MS);

    unlisten = await listen(STATE_EVENT, refreshVisible).catch(() => undefined);
  });

  onBeforeUnmount(() => {
    clearInterval(timer);
    unlisten?.();
  });

  return { snapshot, history, error, busy, refresh, run };
}
