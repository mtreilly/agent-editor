import { test, expect } from '@playwright/test'

test('settings providers page shows global default provider control', async ({ page }) => {
  await page.goto('/settings/providers')
  await expect(page.getByRole('heading', { name: 'Providers' })).toBeVisible()
  await expect(page.getByText('Global default provider')).toBeVisible()
})

