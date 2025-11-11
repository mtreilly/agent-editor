import { test, expect } from '@playwright/test'

test.describe('Providers settings hints', () => {
  test('openrouter inputs expose titles and aria-describedby', async ({ page }) => {
    await page.goto('/settings/providers')

    // Locate the openrouter row
    const row = page.getByRole('row', { name: /openrouter/i })
    await expect(row).toBeVisible()

    const apiKeyInput = row.locator('input[type="password"]')
    await expect(apiKeyInput).toHaveAttribute('aria-describedby', 'hint-openrouter-apikey')
    await expect(apiKeyInput).toHaveAttribute('title')
    const apiKeyHint = page.locator('#hint-openrouter-apikey')
    await expect(apiKeyHint).toBeVisible()

    const modelInput = row.locator('input[type="text"]')
    await expect(modelInput).toHaveAttribute('aria-describedby', 'hint-openrouter-model')
    await expect(modelInput).toHaveAttribute('title')
    const modelHint = page.locator('#hint-openrouter-model')
    await expect(modelHint).toBeVisible()
  })
})

