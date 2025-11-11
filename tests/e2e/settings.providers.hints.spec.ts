import { test, expect } from '@playwright/test'

test.describe('Settings: Provider Hints', () => {
  test('shows disabled/missing key badges for openrouter', async ({ page }) => {
    await page.goto('/settings/providers')
    const row = page.getByRole('row').filter({ hasText: 'openrouter' })
    await expect(row).toBeVisible()
    // By default in web stub: openrouter is disabled; badge should be visible
    await expect(row.getByText(/Disabled/i)).toBeVisible()
  })
})

