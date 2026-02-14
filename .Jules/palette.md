# Palette's Journal - Critical Learnings

This journal tracks unique UX and accessibility patterns, challenges, and solutions found in this codebase.
## 2024-05-23 - Smart Duration Selection
**Learning:** Flat lists with > 50 options (like minute-by-minute duration pickers) are overwhelming. Grouping options by scale (Minutes vs Hours) and using variable granularity (5m for short, 15m/30m for long) significantly improves scanability without sacrificing utility.
**Action:** Always group large select lists with `<optgroup>` and consider non-linear scales for range-based inputs.

## 2024-05-23 - Custom Scroll Pickers Accessibility
**Learning:** Custom scroll-based pickers (like time/duration wheels) are often inaccessible to keyboard and screen reader users if implemented as simple divs with overflow.
**Action:** Always add `tabIndex={0}`, `role="listbox"`, `aria-label`, and ensure items have `role="option"` and `aria-selected`. crucially, add `onClick` handlers for mouse users who can't scroll precisely or prefer clicking.

## 2024-05-22 - Inconsistent Date/Time Pickers
**Learning:** The application uses two different patterns for event creation and editing. `CreateEvent` uses a custom scroll-based picker, while `EventForm` (used for editing) uses standard HTML5 inputs.
**Action:** When unifying UI or adding features, consider which pattern to standardize on. The custom picker is more touch-friendly but less accessible than native inputs.

## 2024-05-24 - Keyboard Navigation for Custom Pickers
**Learning:** Custom scroll pickers with `tabIndex={0}` are focusable but NOT operable with keyboard unless `onKeyDown` handlers (Arrow Up/Down) are explicitly added to hijack scroll/selection. Mouse `onClick` handlers are insufficient for keyboard accessibility.
**Action:** Always implement `onKeyDown` handlers (ArrowUp/ArrowDown) for custom listbox components to ensure they are fully operable without a mouse.
