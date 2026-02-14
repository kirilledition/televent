## 2026-01-25 - Form Label Accessibility Gap
**Learning:** Found multiple form inputs using `<div>` wrappers for labels without `htmlFor`/`id` association, which breaks screen reader support and click-to-focus behavior. This seems to be a common pattern in the initial codebase.
**Action:** When auditing forms, immediately check for `htmlFor` attributes on labels and corresponding `id`s on inputs.

## 2026-02-01 - Modal Accessibility Pattern
**Learning:** Custom modal implementation lacked core accessibility features: no Escape key support and missing ARIA dialog roles.
**Action:** Ensure all custom modals implement `role="dialog"`, `aria-modal="true"`, and an Escape key listener for keyboard dismissal.

## 2026-10-24 - Interactive List Item Accessibility
**Learning:** Interactive list items implemented as clickable `<div>`s (`onClick`) without `role="button"`, `tabIndex={0}`, or keyboard handlers (`onKeyDown`) are invisible to keyboard users. This is a critical barrier for keyboard navigation in list-heavy interfaces.
**Action:** When implementing clickable list items, always ensure they are keyboard accessible by adding `role="button"`, `tabIndex={0}`, and `onKeyDown` handlers for Enter/Space, or use semantic `<button>` elements.

## 2024-05-23 - Smart Duration Selection
**Learning:** Flat lists with > 50 options (like minute-by-minute duration pickers) are overwhelming. Grouping options by scale (Minutes vs Hours) and using variable granularity (5m for short, 15m/30m for long) significantly improves scanability without sacrificing utility.
**Action:** Always group large select lists with <optgroup> and consider non-linear scales for range-based inputs.

## 2024-05-23 - Custom Scroll Pickers Accessibility
**Learning:** Custom scroll-based pickers (like time/duration wheels) are often inaccessible to keyboard and screen reader users if implemented as simple divs with overflow.
**Action:** Always add tabIndex={0}, role="listbox", aria-label, and ensure items have role="option" and aria-selected. crucially, add onClick handlers for mouse users who can't scroll precisely or prefer clicking.

## 2024-05-22 - Inconsistent Date/Time Pickers
**Learning:** The application uses two different patterns for event creation and editing. CreateEvent uses a custom scroll-based picker, while EventForm (used for editing) uses standard HTML5 inputs.
**Action:** When unifying UI or adding features, consider which pattern to standardize on. The custom picker is more touch-friendly but less accessible than native inputs.
