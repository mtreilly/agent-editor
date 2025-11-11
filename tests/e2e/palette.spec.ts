import { test, expect } from '@playwright/test'

test('command palette toggles and shows input', async ({ page, browserName }) => {
  await page.goto('/')
  // Open with Ctrl/Cmd+K
  const isMac = process.platform === 'darwin'
  if (isMac) {
    await page.keyboard.down('Meta')
    await page.keyboard.press('KeyK')
    await page.keyboard.up('Meta')
  } else {
    await page.keyboard.down('Control')
    await page.keyboard.press('KeyK')
    await page.keyboard.up('Control')
  }
  await expect(page.getByLabel('Command palette input')).toBeVisible()
})

