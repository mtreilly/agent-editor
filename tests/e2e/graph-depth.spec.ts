import { test, expect } from '@playwright/test'

test('graph page shows neighbor depth control', async ({ page }) => {
  await page.goto('/graph/dummy')
  await expect(page.getByRole('heading', { name: 'Graph' })).toBeVisible()
  await expect(page.getByLabel('Depth')).toBeVisible()
  const select = page.getByLabel('Depth')
  await expect(select).toHaveValue('1')
  await select.selectOption('2')
  await expect(select).toHaveValue('2')
})

