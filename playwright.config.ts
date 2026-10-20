import { defineConfig, devices } from '@playwright/test';
import { defineBddConfig } from 'playwright-bdd';
import { ALLURE_RESULTS_DIR } from './tests/e2e/globalSetup';

const testDir = defineBddConfig({
  features: 'tests/e2e/features/**/*.feature',
  steps: ['tests/e2e/steps/**/*.ts', 'tests/e2e/fixtures/desktop.ts'],
  outputDir: 'tests/e2e/.features-gen',
});

export default defineConfig({
  testDir,
  fullyParallel: true,
  workers: 4,
  globalSetup: './tests/e2e/globalSetup.ts',
  reporter: [['list'], ['allure-playwright', { resultsDir: ALLURE_RESULTS_DIR, detail: false }]],
  use: {
    baseURL: 'http://127.0.0.1:1421',
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: 'npm run dev -- --host 127.0.0.1 --port 1421',
    url: 'http://127.0.0.1:1421',
    reuseExistingServer: !process.env.CI,
  },
});
