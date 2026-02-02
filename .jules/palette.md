## 2026-01-25 - Form Label Accessibility Gap
**Learning:** Found multiple form inputs using `<div>` wrappers for labels without `htmlFor`/`id` association, which breaks screen reader support and click-to-focus behavior. This seems to be a common pattern in the initial codebase.
**Action:** When auditing forms, immediately check for `htmlFor` attributes on labels and corresponding `id`s on inputs.

## 2026-02-01 - Modal Accessibility Pattern
**Learning:** Custom modal implementation lacked core accessibility features: no Escape key support and missing ARIA dialog roles.
**Action:** Ensure all custom modals implement `role="dialog"`, `aria-modal="true"`, and an Escape key listener for keyboard dismissal.

## 2026-10-24 - Interactive List Item Accessibility
**Learning:** Interactive list items implemented as clickable `<div>`s (`onClick`) without `role="button"`, `tabIndex={0}`, or keyboard handlers (`onKeyDown`) are invisible to keyboard users. This is a critical barrier for keyboard navigation in list-heavy interfaces.
**Action:** When implementing clickable list items, always ensure they are keyboard accessible by adding `role="button"`, `tabIndex={0}`, and `onKeyDown` handlers for Enter/Space, or use semantic `<button>` elements.
