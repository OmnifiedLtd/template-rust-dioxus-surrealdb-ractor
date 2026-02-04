const { test, expect } = require('@playwright/test');

test('admin can pause/resume queues and inspect job payloads', async ({ page }) => {
  await page.goto('/admin', { waitUntil: 'domcontentloaded' });
  await expect(page.getByRole('heading', { name: 'Job Queue Admin' })).toBeVisible({
    timeout: 30000,
  });

  const queueCard = page.locator('.queue-card').first();
  await expect(queueCard).toBeVisible();

  const stateBadge = queueCard.locator('.state-badge');
  await expect(stateBadge).toBeVisible();

  const pauseButton = queueCard.getByRole('button', { name: 'Pause' });
  const resumeButton = queueCard.getByRole('button', { name: 'Resume' });
  const initialState = (await stateBadge.textContent())?.trim();

  if (initialState === 'Running') {
    await pauseButton.click();
    await expect(stateBadge).toHaveText('Paused');
    await resumeButton.click();
    await expect(stateBadge).toHaveText('Running');
  } else {
    await resumeButton.click();
    await expect(stateBadge).toHaveText('Running');
    await pauseButton.click();
    await expect(stateBadge).toHaveText('Paused');
    await resumeButton.click();
    await expect(stateBadge).toHaveText('Running');
  }

  await queueCard.click();

  await page.getByRole('button', { name: '+ New Job' }).click();
  await expect(page.getByRole('heading', { name: 'Create New Job' })).toBeVisible();

  const selects = page.locator('.create-job-form select');
  await selects.nth(0).selectOption('sleep');

  const message = `hello-e2e-${Date.now()}`;
  const payload = JSON.stringify({ seconds: 10, note: message });

  await page.locator('.create-job-form textarea').fill(payload);
  await page.getByRole('button', { name: 'Create Job' }).click();

  const rows = page.locator('.job-row');
  await expect(rows.first()).toBeVisible();

  const payloadJson = page.locator('.job-detail-panel .payload-json');
  let found = false;
  const rowCount = await rows.count();

  for (let i = 0; i < rowCount; i += 1) {
    await rows.nth(i).click();
    await expect(payloadJson).toBeVisible();
    const text = await payloadJson.textContent();
    if (text && text.includes(message)) {
      found = true;
      break;
    }
  }

  expect(found).toBeTruthy();
});
