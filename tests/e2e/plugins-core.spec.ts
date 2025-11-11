import { test, expect } from '@playwright/test'

test('plugins page shows core plugins panel', async ({ page }) => {
  await page.goto('/plugins')
  await expect(page.getByRole('heading', { name: /Plugins/i })).toBeVisible()
  await expect(page.getByRole('heading', { name: /Core Plugins/i })).toBeVisible()
  await expect(page.getByRole('button', { name: /Spawn Echo/i })).toBeVisible()
})

