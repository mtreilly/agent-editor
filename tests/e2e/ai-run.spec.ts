import { test, expect } from '@playwright/test'

test('doc page Run AI renders provider header', async ({ page }) => {
  await page.goto('/doc/dummy')
  // Click Run AI (uses web stub for ai_run). Target exact label to avoid @Anchor button.
  const runBtn = page.getByTestId('run-ai-btn')
  await expect(runBtn).toBeEnabled()
  await runBtn.click()
  // Expect Provider header to show up above output
  await expect(page.getByText(/Provider:/)).toBeVisible()
})
