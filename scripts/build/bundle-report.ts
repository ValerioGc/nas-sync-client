import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Plugin } from 'vite';

const root = fileURLToPath(new URL('../../', import.meta.url));

/**
 * Record the shipped module graph for the desktop bundle checks.
 */
export function bundleReport(): Plugin {
  return {
    name: 'nas-sync-client:bundle-report',
    apply: 'build',
    generateBundle(_options, bundle) {
      const modules = [
        ...new Set(
          Object.values(bundle).flatMap((item) =>
            item.type === 'chunk' ? Object.keys(item.modules) : [],
          ),
        ),
      ];

      const report = resolve(root, 'build-reports/tauri.json');
      mkdirSync(dirname(report), { recursive: true });
      writeFileSync(
        report,
        JSON.stringify(
          { modules: modules.map((id) => relative(root, id).replaceAll('\\', '/')) },
          null,
          2,
        ),
      );
    },
  };
}
