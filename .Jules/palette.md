# Palette's Journal - Critical Learnings

This journal tracks unique UX and accessibility patterns, challenges, and solutions found in this codebase.

## 2024-05-22 - Inconsistent Date/Time Pickers
**Learning:** The application uses two different patterns for event creation and editing. `CreateEvent` uses a custom scroll-based picker, while `EventForm` (used for editing) uses standard HTML5 inputs.
**Action:** When unifying UI or adding features, consider which pattern to standardize on. The custom picker is more touch-friendly but less accessible than native inputs.

## 2024-05-23 - Custom Scroll Pickers Accessibility
**Learning:** Custom scroll-based pickers (like time/duration wheels) are often inaccessible to keyboard and screen reader users if implemented as simple divs with overflow.
**Action:** Always add `tabIndex={0}`, `role="listbox"`, `aria-label`, and ensure items have `role="option"` and `aria-selected`. crucially, add `onClick` handlers for mouse users who can't scroll precisely or prefer clicking.
