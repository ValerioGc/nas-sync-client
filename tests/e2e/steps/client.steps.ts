import { expect } from '@playwright/test';
import { createBdd } from 'playwright-bdd';
import { test } from '../fixtures/desktop';

const { Given, When, Then } = createBdd(test);

const connected = (page: import('@playwright/test').Page) =>
  page.addInitScript(() => {
    window.__clientFixture.snapshot.link = {
      name: 'studio-pc',
      host: '192.168.1.15',
      port: 443,
      fingerprint: 'ab'.repeat(32),
    };
  });

Given('the client is open without a NAS', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByTestId('connect_wizard')).toBeVisible();
});

When('I look for the NAS on the network and select it', async ({ page }) => {
  await page.getByRole('button', { name: 'Search' }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: 'Select' }).click();
  await expect(page.getByTestId('wizard_host')).toHaveValue('192.168.1.15');
});

When('I test the connection', async ({ page }) => {
  await page.getByTestId('wizard_test').click();
});

Then('the fingerprint of the NAS is shown', async ({ page }) => {
  await expect(page.getByTestId('wizard_fingerprint')).toContainText('AB:AB:AB');
});

When('I confirm the fingerprint', async ({ page }) => {
  await page.getByTestId('wizard_trust').click();
  await expect(page.getByTestId('wizard_name')).toHaveValue('studio-pc');
});

When('I sign in with a wrong password', async ({ page }) => {
  await page.getByTestId('wizard_username').fill('admin');
  await page.getByTestId('wizard_password').fill('wrong');
  await page.getByTestId('wizard_connect').click();
});

Then('the client says the credentials are wrong', async ({ page }) => {
  await expect(page.getByTestId('wizard_error')).toHaveText('Wrong username or password');
});

When('I sign in with the right password', async ({ page }) => {
  await page.getByTestId('wizard_password').fill('secret');
  await page.getByTestId('wizard_connect').click();
});

Then('the synced folders of the NAS are shown', async ({ page }) => {
  await expect(page.getByText('No synced folders')).toBeVisible();
  await expect(page.getByTestId('status_badge')).toHaveText('Synced');
});

Given('the client is connected to the NAS', async ({ page }) => {
  await connected(page);
  await page.goto('/');
  await expect(page.getByTestId('status_panel')).toBeVisible();
});

When('I create a sync of my Photos folder with the NAS', async ({ page }) => {
  await page.getByTestId('new_sync').click();
  const dialog = page.getByRole('dialog');
  await dialog.getByTestId('rule_browse').click();
  await dialog.getByTestId('nas_folder').click();
  await dialog.getByTestId('nas_folder').click();
  await expect(dialog.getByTestId('nas_selected')).toHaveText('Data / Photos');
  await dialog.getByTestId('rule_save').click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

Then('the sync is listed as synced', async ({ page }) => {
  const row = page.getByTestId('sync_row');
  await expect(row).toContainText('/home/me/Photos');
  await expect(row).toContainText('Data/Photos');
  await expect(row).toContainText('Synced');
});

When('I pause the sync', async ({ page }) => {
  await page.getByRole('button', { name: 'Pause Photos' }).click();
});

Then('the sync is listed as paused', async ({ page }) => {
  await expect(page.getByTestId('sync_row')).toContainText('Paused');
});

When('I remove the sync', async ({ page }) => {
  await page.getByRole('button', { name: 'Remove Photos' }).click();
  await page.getByTestId('confirm_action').click();
});

Then('no folder is synced', async ({ page }) => {
  await expect(page.getByText('No synced folders')).toBeVisible();
});

When('the NAS stops answering', async ({ page }) => {
  await page.evaluate(() => {
    window.__clientFixture.snapshot.problem = 'nas_unreachable';
    window.__emitClientState();
  });
});

Then('the client asks to check the network connection', async ({ page }) => {
  await expect(page.getByTestId('status_badge')).toHaveText('Not connected');
  await expect(page.getByRole('status')).toContainText('Check the network connection');
});

When('I change the address of the NAS in the options', async ({ page }) => {
  await page.getByTestId('open_options').click();
  await page.getByTestId('options_host').fill('nas-studio.local');
  await page.getByTestId('options_move').click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

Then('the new address is shown', async ({ page }) => {
  await expect(page.getByTestId('status_panel')).toContainText('nas-studio.local');
});

When('I disconnect the NAS in the options', async ({ page }) => {
  await page.getByTestId('open_options').click();
  await page.getByTestId('options_disconnect').click();
  await page.getByTestId('confirm_action').click();
});

Then('the connection wizard is shown', async ({ page }) => {
  await expect(page.getByTestId('connect_wizard')).toBeVisible();
});

Given('the tray summary is open with recent files', async ({ page }) => {
  await connected(page);
  await page.addInitScript(() => {
    window.__clientFixture.history = [
      {
        time: '2026-10-16T10:00:00Z',
        folder: 'f-1',
        path: 'docs/report.pdf',
        action: 'modified',
        direction: 'upload',
        conflict: false,
      },
    ];
  });
  await page.goto('/#mini');
});

Then('the summary shows the state and the recent files', async ({ page }) => {
  await expect(page.getByTestId('mini_view')).toBeVisible();
  await expect(page.getByTestId('history_entry')).toContainText('report.pdf');
});

When('I pause the sync from the summary menu', async ({ page }) => {
  await page.getByTestId('dropdown_toggle').click();
  await page.getByRole('menuitem', { name: 'Pause sync' }).click();
});

Then('the summary shows the sync paused', async ({ page }) => {
  await expect(page.getByTestId('status_badge')).toHaveText('Sync paused');
});
