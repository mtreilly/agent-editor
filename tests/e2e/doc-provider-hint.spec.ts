import { test, expect } from '@playwright/test'

test('doc page shows provider hint when disabled', async ({ page }) => {
  // The web stub returns allowed=false when doc id contains 'disabled'
  await page.goto('/doc/disabled')
  const runBtn = page.getByTestId('run-ai-btn')
  await expect(runBtn).toBeDisabled()
  await expect(page.getByText(/Provider not allowed/i)).toBeVisible()
})

