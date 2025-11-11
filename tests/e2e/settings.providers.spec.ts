import { test, expect } from '@playwright/test'

test.describe('Settings: Providers', () => {
  test('lists providers and sets global default', async ({ page }) => {
    await page.goto('/settings/providers')

    await expect(page.getByRole('heading', { name: /Providers/i })).toBeVisible()

    // Table appears with at least one row
    await expect(page.getByRole('table')).toBeVisible()

    // Enable openrouter (if present)
    const row = page.getByRole('row').filter({ hasText: 'openrouter' })
    const toggleBtn = row.getByRole('button', { name: /Enable|Disable/ })
    if (await toggleBtn.isVisible()) {
      await toggleBtn.click()
    }

    // Change global default to openrouter
    const select = page.locator('select')
    await select.selectOption('openrouter')
    await expect(select).toHaveValue('openrouter')

    // OpenRouter model input present and usable
    const modelInput = page.getByPlaceholder(/openrouter\/auto/i)
    await modelInput.fill('openrouter/auto')
    const saveModel = page.getByRole('button', { name: /Save Model/i })
    await saveModel.click()
  })
})
