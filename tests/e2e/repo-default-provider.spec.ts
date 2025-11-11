import { test, expect } from '@playwright/test'

test('repo page shows default provider selector', async ({ page }) => {
  await page.goto('/repo')
  await expect(page.getByRole('heading', { name: 'Repositories' })).toBeVisible()
  await expect(page.getByText('Default Provider')).toBeVisible()
  await expect(page.getByRole('button', { name: /Set/ })).toBeVisible()
  await expect(page.getByText(/Effective/)).toBeVisible()
})
