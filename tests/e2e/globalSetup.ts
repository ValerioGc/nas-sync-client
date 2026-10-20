import { rm } from 'node:fs/promises';

export const ALLURE_RESULTS_DIR = 'tests/e2e/allure-results';

export default async function globalSetup() {
  await rm(ALLURE_RESULTS_DIR, { recursive: true, force: true });
}
