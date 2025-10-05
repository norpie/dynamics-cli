# Migration App TODO

## Next Steps (Priority Order)

### High Priority - User Experience
1. **Hide matched filtering** - Toggle to show only unmatched items, critical for focusing on work remaining
2. **Mirrored selection** - Highlight corresponding item on other side when selecting a match
3. **Field value display from examples** - Show actual data to verify mappings are correct

### Medium Priority - Quality of Life
4. **Visual prefix indicator** - Distinguish `[Prefix]` matches from `[Exact]` matches visually
5. **Undo/redo mapping actions** - Recover from mistakes without restarting
6. **Sorting modes** - "Matches first" to see progress, alphabetical for scanning
7. **Match confidence scores** - Numeric confidence in fuzzy/partial matches

### Low Priority - Advanced Features
8. **CRM shortcuts** - Open fields/entities/records directly in browser
9. **Context menu** - Right-click actions for mapping operations
10. **Display name vs logical name toggle** - Switch between technical and user-friendly names

## Infrastructure (Setup)
- [x] Create entity comparison app module structure
- [x] Set up API metadata models (FieldMetadata, EntityMetadata, etc.)
- [x] Set up app state models (Side, ActiveTab, ExamplesState, MatchInfo, etc.)
- [x] Create TreeItem implementations (FieldNode, RelationshipNode, ViewNode, FormNode, ContainerNode)
- [x] Implement tab system (Fields, Relationships, Views, Forms)
- [x] Add tab switching with 1-4 keyboard shortcuts
- [x] Create tab indicator in status bar
- [x] Add separator between title and status in header

## Matching System
- [x] Basic field matching (exact name, prefix transformation)
- [x] Relationship matching
- [x] Hierarchical path-based matching for Forms/Views tabs
- [x] Container matching with aggregated status (FullMatch, Mixed, NoMatch)
- [x] Field matching within containers using normal logic (exact, prefix)
- [x] Manual mapping support for fields, relationships, and containers
- [x] Database persistence for manual mappings (SQLite)
- [x] Load saved mappings on app init
- [x] Type mismatch detection (yellow color)
- [x] Reverse match mapping for target tree
- [x] Container match info display with arrows and labels

## Data Processing
- [x] Filter out `_*_value` virtual lookup fields
- [x] Consolidate lookup field metadata (detect lookups from _value pattern)
- [x] Mark base fields as Lookup type when _*_value field exists
- [ ] Show formatted display names in examples for lookup fields

## Features to Implement in Tree Component Context

### **Field Value Display from Examples**
- [ ] Show live record data inline next to field names when examples mode enabled
- [ ] Format: `field_name: "actual value from API"`
- [ ] Handle missing data gracefully (`[no data]`, `[no value]`, `[null]`, `[empty]`)

### **Match Status Indicators**
- [x] Show mapping type badge/icon: `[Exact]`, `[Prefix]`, `[Manual]`, `[TypeMismatch]`
- [x] Color code by match type (green=match, yellow=type mismatch, red=no match)
- [ ] Display match confidence score

### **Hide Matched Filtering**
- [ ] Filter out matched items from tree when toggle enabled
- [ ] Show only unmatched fields/items
- [ ] Toggle for containers vs just fields

### **Field Mapping Actions**
- [x] Create manual mapping (m key with selections on both sides)
- [x] Delete manual mapping (d key)
- [x] Show which target field a source maps to (arrow with target name)
- [x] Display match type label for containers and fields
- [x] **Preserve tree position after creating/deleting manual mapping**
- [ ] Undo/redo mapping actions

### **Prefix Mapping Effects**
- [x] Prefix transformations applied in hierarchical matching
- [ ] Show transformed name alongside original in display
- [x] Visual indicator when prefix mapping applies vs exact match (shows [Prefix] label)

### **Mirrored Selection**
- [ ] When selecting matched pair on one side, highlight corresponding item on other side
- [ ] Sync scroll position between matched items on source/target

### **Context Menu / Actions**
- [ ] Create manual mapping (requires selection on both sides)
- [ ] Delete manual mapping
- [ ] Add to examples (for specific records)
- [ ] Show mapping details

### **Node Metadata Display**
- [x] Field type information (shown in angle brackets)
- [x] Required/optional status (red asterisk for required)
- [ ] Display name vs logical name
- [x] Related entity information (for lookups/relationships)

### **Manual API Refresh**
- [x] Re-fetch metadata/examples from API on demand (F5 keybinding)
- [x] Preserve current tree position (selected item, scroll offset, expanded nodes)
- [x] Show loading indicator during refresh

### **Sorting Modes**
- [ ] **Matches first**: Matched items at top (alphabetically), unmatched at bottom (alphabetically)
- [ ] Preserve sort order across refreshes
- [ ] Update sort when match status changes

### **CRM Shortcuts**
- [ ] Open field in CRM customization page
- [ ] Open entity in CRM
- [ ] Open example record in CRM
- [ ] Copy CRM URLs to clipboard
