## 2024-05-24 - Breaking Reference Equality in React Components
**Learning:** Deriving data (like `const events = data.map(...)`) directly in the render body creates a new array reference on every render. This completely defeats the purpose of `useMemo` in child components that depend on this prop (e.g., `EventList`), causing them to re-compute expensive operations (sorting/grouping) unnecessarily.
**Action:** Always wrap derived data transformations in `useMemo` when passing them to memoized child components to preserve reference stability.
