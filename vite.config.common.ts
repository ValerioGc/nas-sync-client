import { fileURLToPath, URL } from 'node:url';

import vue from '@vitejs/plugin-vue';
import type { UserConfig } from 'vite';
import { stripTestIds, stripTestIdsNodeTransform } from './scripts/build/strip-test-ids.ts';
import { bundleReport } from './scripts/build/bundle-report.ts';

import combineSelectors from 'postcss-combine-duplicated-selectors';
import { purgeCSSPlugin } from '@fullhuman/postcss-purgecss';
import autoprefixer from 'autoprefixer';
import cssnano from 'cssnano';

export const commonConfig: UserConfig = {
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@tests': fileURLToPath(new URL('./tests', import.meta.url)),
    },
  },
};

export function frontendConfig(command: 'build' | 'serve'): UserConfig {
  return {
    ...commonConfig,
    base: '/',
    resolve: {
      alias: {
        '@': fileURLToPath(new URL('./src', import.meta.url)),
      },
    },
    css: {
      preprocessorOptions: {
        scss: { additionalData: `@use '@/styles/global' as *;\n` },
      },
      postcss: {
        plugins: [
          autoprefixer(),
          ...(command === 'build'
            ? [
                combineSelectors({ removeDuplicatedProperties: true }),
                purgeCSSPlugin({
                  content: ['./index.html', './src/**/*.vue', './src/**/*.ts', './src/**/*.scss'],
                  defaultExtractor: (content) => content.match(/[\w-/:]+(?<!:)/g) ?? [],
                }),
                cssnano({ preset: 'default' }),
              ]
            : []),
        ],
      },
    },
    plugins: [
      bundleReport(),
      vue(
        command === 'build'
          ? { template: { compilerOptions: { nodeTransforms: [stripTestIdsNodeTransform] } } }
          : {},
      ),
      ...(command === 'build' ? [stripTestIds()] : []),
    ],
    build: { outDir: 'dist/tauri', emptyOutDir: true, sourcemap: false },
  };
}
