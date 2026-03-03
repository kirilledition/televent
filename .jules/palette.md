## 2026-01-25 - Form Label Accessibility Gap
**Learning:** Found multiple form inputs using `<div>` wrappers for labels without `htmlFor`/`id` association, which breaks screen reader support and click-to-focus behavior. This seems to be a common pattern in the initial codebase.
**Action:** When auditing forms, immediately check for `htmlFor` attributes on labels and corresponding `id`s on inputs.

## 2026-02-01 - Modal Accessibility Pattern
**Learning:** Custom modal implementation lacked core accessibility features: no Escape key support and missing ARIA dialog roles.
**Action:** Ensure all custom modals implement `role="dialog"`, `aria-modal="true"`, and an Escape key listener for keyboard dismissal.

## 2026-10-24 - Interactive List Item Accessibility
**Learning:** Interactive list items implemented as clickable `<div>`s (`onClick`) without `role="button"`, `tabIndex={0}`, or keyboard handlers (`onKeyDown`) are invisible to keyboard users. This is a critical barrier for keyboard navigation in list-heavy interfaces.
**Action:** When implementing clickable list items, always ensure they are keyboard accessible by adding `role="button"`, `tabIndex={0}`, and `onKeyDown` handlers for Enter/Space, or use semantic `<button>` elements.

## 2026-10-25 - Undefined Tailwind Utility Classes for Focus States
**Learning:** Forms and interactive components were using non-existent/undefined Tailwind utility classes (like `focus:border-primary` and `focus:ring-primary`) or missing focus states entirely, causing them to lack keyboard focus indicators. This makes the application completely inaccessible for keyboard users and those with visual impairments.
**Action:** When working on UI components, do not assume composite or theme-specific classes exist unless explicitly defined. Always use explicit, accessible focus rings leveraging existing CSS variables (e.g., `focus-visible:ring-2 focus-visible:ring-[var(--ctp-mauve)] focus-visible:border-transparent`).
