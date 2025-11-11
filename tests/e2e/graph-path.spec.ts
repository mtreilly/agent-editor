import { test, expect } from '@playwright/test'

test('graph path tool computes and lists path', async ({ page }) => {
  await page.goto('/graph/start-doc')
  await expect(page.getByRole('heading', { name: 'Graph' })).toBeVisible()
  const input = page.getByPlaceholder('Target doc id or slug')
  await input.fill('end-doc')
  await page.getByRole('button', { name: 'Compute' }).click()
  // With web stubs, path is [start, end]. Titles equal ids in stubs
  const items = page.locator('ol li')
  await expect(items.nth(0)).toHaveText(/start-doc/)
  await expect(items.nth(1)).toHaveText(/end-doc/)
})

