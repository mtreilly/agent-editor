import { test, expect } from '@playwright/test'

test.describe('Command Palette', () => {
  test('opens and supports keyboard + ARIA', async ({ page }) => {
    await page.goto('/')

    // Try Meta+K, fallback to Control+K
    await page.keyboard.press('Meta+K').catch(() => {})
    let visible = await page.getByRole('listbox').first().isVisible().catch(() => Promise.resolve(false))
    if (!visible) {
      await page.keyboard.press('Control+K').catch(() => {})
      visible = await page.getByRole('listbox').first().isVisible().catch(() => Promise.resolve(false))
    }
    if (!visible) {
      await page.evaluate(() => {
        const ev = new KeyboardEvent('keydown', { key: 'k', metaKey: true })
        window.dispatchEvent(ev)
      })
    }

    const listbox = page.locator('#palette-listbox')
    await expect(listbox).toBeVisible()

    // Should have options or the noMatches option
    const options = listbox.getByRole('option')
    await expect(options.first()).toBeVisible()

    const before = await listbox.getAttribute('aria-activedescendant')
    await page.keyboard.press('ArrowDown')
    const after = await listbox.getAttribute('aria-activedescendant')
    expect(before).not.toEqual(after)
  })
})
