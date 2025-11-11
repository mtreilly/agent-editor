import { test, expect } from '@playwright/test'

test.describe('Doc AI accessibility', () => {
  test('Run AI disabled exposes aria-describedby and hint when provider not allowed', async ({ page }) => {
    // Using stub: any doc id containing 'disabled' yields allowed=false in aiProviderResolve
    await page.goto('/doc/disabled-123')

    const runBtn = page.getByTestId('run-ai-btn')
    await expect(runBtn).toBeVisible()
    await expect(runBtn).toBeDisabled()
    await expect(runBtn).toHaveAttribute('aria-describedby', 'provider-disabled-hint')
    await expect(page.locator('#provider-disabled-hint')).toBeVisible()
  })

  test('Run AI enabled has no provider-disabled describedby', async ({ page }) => {
    await page.goto('/doc/ok-123')
    const runBtn = page.getByTestId('run-ai-btn')
    await expect(runBtn).toBeVisible()
    await expect(runBtn).toBeEnabled()
    await expect(runBtn).not.toHaveAttribute('aria-describedby', /provider-disabled-hint/)
  })
})

