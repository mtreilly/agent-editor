# A11Y Audit — agent-editor (snapshot)

Status of key views against the A11Y checklist (docs/guides/A11Y.md).

Legend: [OK] complete, [TODO] improvements pending

- Home (/): [OK] semantics/roles
- Search (/search): [OK] listbox ARIA, keyboard navigation
- Repo (/repo): [OK] forms and labels; [TODO] Add aria-live region for scan progress
- Doc (/doc/$id): [OK] headings; [OK] buttons with labels; [OK] Run AI disabled state with hint; [TODO] Consider aria-live for AI output
- Graph (/graph/$id): [OK] headings; [OK] controls labelled
- Settings → Providers: [OK] badges and hints; [OK] inputs and buttons; [TODO] Summarize state via aria-describedby for model field
- Plugins (/plugins): [OK] headings and buttons; [TODO] Core controls disabled states get aria-disabled annotations

Update this file as routes are improved.

- [x] Repo scan progress uses aria-live polite
- [x] Doc AI output region uses role=status + aria-live polite
- [x] Provider settings model input has aria-describedby hint
- [x] Plugins panel buttons expose aria-disabled semantics with describedby when disabled
