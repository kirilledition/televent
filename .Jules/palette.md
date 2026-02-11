# Palette's Journal - Critical Learnings

This journal tracks unique UX and accessibility patterns, challenges, and solutions found in this codebase.

## 2024-05-23 - Custom Scroll Pickers Accessibility
**Learning:** Custom scroll-based pickers (like time/duration wheels) are often inaccessible to keyboard and screen reader users if implemented as simple divs with overflow.
**Action:** Always add `tabIndex={0}`, `role="listbox"`, `aria-label`, and ensure items have `role="option"` and `aria-selected`. crucially, add `onClick` handlers for mouse users who can't scroll precisely or prefer clicking.
