import { test, expect } from '@playwright/test'

test('home loads and nav exists', async ({ page }) => {
  await page.goto('/')
  await expect(page.getByText('agent-editor')).toBeVisible()
  await expect(page.getByRole('link', { name: 'Search' })).toBeVisible()
  await expect(page.getByRole('link', { name: 'Repos' })).toBeVisible()
})

test('search route renders', async ({ page }) => {
  await page.goto('/search')
  await expect(page.getByRole('heading', { name: 'Search' })).toBeVisible()
})

test('repo route renders', async ({ page }) => {
  await page.goto('/repo')
  await expect(page.getByRole('heading', { name: 'Add Repository' })).toBeVisible()
})

