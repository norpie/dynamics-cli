# TUI Framework - TODO

## High Priority

### UnifiedCompareApp (Migration Phase 3)
Main comparison screen with 4 tabs for Dynamics 365 field/relationship/view/form comparison.
- [ ] Fields tab with tree view and field mapping (exact, prefix, manual)
- [ ] Relationships tab
- [ ] Views tab
- [ ] Forms tab
- [ ] Examples modal
- [ ] Export to JSON/Excel

**Estimated effort:** 2-3 weeks (~2000-3000 lines)

---

## Low Priority Widgets

### ProgressBar
- [ ] 0.0-1.0 progress value
- [ ] Optional label text
- [ ] Render as `[████████░░░░] 65%`

### Checkbox & RadioGroup
- [ ] Checkbox: `[✓]` / `[ ]` rendering, Space to toggle
- [ ] RadioGroup: `(•)` / `( )` rendering, exclusive selection

### Table
- [ ] Headers, rows, column widths
- [ ] Sortable columns
- [ ] Row selection with `on_select`

### Menu/Dropdown
- [ ] MenuItem list with labels
- [ ] Keyboard shortcuts display
- [ ] Disabled state
- [ ] Separator support

---

## Performance Optimizations

### Virtual Scrolling (List widget)
- [ ] Render only visible items (critical for 10,000+ item lists)
- [ ] Currently all items are rendered

### View Memoization
- [ ] Cache view() results when state unchanged
- [ ] Hash-based state change detection
- [ ] Skip rendering unchanged subtrees

---

## Layout & Styling

### Additional Layout Constraints
- [ ] `Percentage(u16)` - e.g., 30% of container width
- [ ] `Max(u16)` - at most N units
- [ ] `Ratio(u16)` - for proportional layouts (1:2:1)

### Style Composition
- [ ] `Style::merge()` - compose multiple styles
- [ ] Pseudo-states for Button (normal/hover/active/disabled)
- [ ] Style inheritance from parent containers

---

## Developer Experience

### Debug Overlay
- [ ] F12 to toggle overlay
- [ ] Show element boundaries (colored boxes)
- [ ] Display FocusIds
- [ ] Show layout constraints
- [ ] Highlight interaction areas
- [ ] Display current focus state

### Performance Profiler
- [ ] Measure frame render time
- [ ] Identify slow elements
- [ ] Log breakdown (view() vs render())

---

## Nice to Have

- [ ] Hot reload (watch source files, recompile without restart)
- [ ] High-contrast theme variant
- [ ] Customizable keybindings (config file)
- [ ] Screen reader support (aria labels, audio cues)
