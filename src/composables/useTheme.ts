import { ref } from 'vue';
import type { Theme } from '@/types';

const theme = ref<Theme>('dark');

function applyTheme() {
  document.documentElement.dataset.theme = theme.value;
  document.documentElement.style.colorScheme = theme.value;
}

export function setTheme(nextTheme: Theme) {
  theme.value = nextTheme;
  applyTheme();
}

export function useTheme() {
  return { theme, setTheme };
}
