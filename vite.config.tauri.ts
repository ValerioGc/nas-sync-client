import { defineConfig } from 'vite';
import { frontendConfig } from './vite.config.common.ts';

const host = process.env.TAURI_DEV_HOST;
export default defineConfig(({ command }) => ({
  ...frontendConfig(command),
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_'],
  server: {
    port: 1420,
    strictPort: true,
    host: host || '127.0.0.1',
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/backend/**'] },
  },
}));
