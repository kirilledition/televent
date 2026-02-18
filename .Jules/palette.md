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

## 2024-05-24 - Keyboard Navigation in Custom Pickers
**Learning:** `tabIndex={0}` and `role="listbox"` are insufficient for accessible scroll pickers. Users expect standard listbox interaction (ArrowUp/ArrowDown) to change selection, not just scroll the container.
**Action:** Implement `onKeyDown` handlers for Arrow keys to explicitly update state and selection, in addition to existing scroll/click logic.

## 2024-05-25 - Theming Shadcn Components
**Learning:** Shadcn UI components rely on standard CSS variables (like `--background`) which may not be defined in custom-themed apps (e.g. using Catppuccin variables directly).
**Action:** When adding Shadcn components to a custom-themed codebase, explicitly override default classes with theme-specific variables (e.g. `bg-[var(--ctp-base)]`) to ensure visual consistency.

## 2024-05-25 - React Portals Event Bubbling
**Learning:** Events from elements inside a React Portal (like `AlertDialogContent`) bubble up to their *React* parent (the component that rendered the Portal), not just their DOM parent. This means clicking inside a modal could trigger click handlers on the component that opened it (e.g. an `EventItem` row).
**Action:** Always add `e.stopPropagation()` to the content container of a Portal-based component if it's rendered inside another interactive element.

## 2024-05-26 - Missing Utility Classes
**Learning:** Some components referenced `btn-primary` and `btn-secondary` classes which were not defined in the CSS, leading to unstyled buttons. This highlights the risk of relying on utility classes without verifying their existence.
**Action:** Always verify custom utility classes exist or use standard Tailwind classes directly in the component to ensure styles are applied correctly.
