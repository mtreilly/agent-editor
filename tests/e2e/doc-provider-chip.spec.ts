import { test, expect } from '@playwright/test'

test('doc page shows effective provider chip', async ({ page }) => {
  await page.goto('/doc/dummy')
  await expect(page.getByText(/Provider:/)).toBeVisible()
})

