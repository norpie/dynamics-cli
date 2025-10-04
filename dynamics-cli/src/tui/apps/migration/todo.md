# Migration App TODO

## Features to Implement in Tree Component Context

### **Field Value Display from Examples**
- [ ] Show live record data inline next to field names when examples mode enabled
- [ ] Format: `field_name: "actual value from API"`
- [ ] Handle missing data gracefully (`[no data]`, `[no value]`, `[null]`, `[empty]`)

### **Match Status Indicators**
- [ ] Show mapping type badge/icon: `[Exact]`, `[Prefix]`, `[Manual]`
- [ ] Color code by match type
- [ ] Display match score

### **Hide Matched Filtering**
- [ ] Filter out matched items from tree when toggle enabled
- [ ] Show only unmatched fields/items

### **Field Mapping Actions**
- [ ] Select source field + target field to create manual mapping
- [ ] Delete existing manual mapping from selected field
- [ ] Show which target field a source maps to (and vice versa)
- [ ] **Preserve tree position after creating/deleting manual mapping**

### **Prefix Mapping Effects**
- [ ] Apply prefix transformations to display names
- [ ] Show transformed name alongside original
- [ ] Indicate when prefix mapping applies

### **Mirrored Selection**
- [ ] When selecting matched pair on one side, highlight corresponding item on other side
- [ ] Sync scroll position between matched items on source/target

### **Context Menu / Actions**
- [ ] Create manual mapping (requires selection on both sides)
- [ ] Delete manual mapping
- [ ] Add to examples (for specific records)
- [ ] Show mapping details

### **Node Metadata Display**
- [ ] Field type information
- [ ] Required/optional status
- [ ] Display name vs logical name
- [ ] Related entity information (for lookups)

### **Manual API Refresh**
- [ ] Re-fetch metadata/examples from API on demand
- [ ] Preserve current tree position (selected item, scroll offset, expanded nodes)
- [ ] Show loading indicator during refresh

### **Sorting Modes**
- [ ] **Matches first**: Matched items at top (alphabetically), unmatched at bottom (alphabetically)
- [ ] Preserve sort order across refreshes
- [ ] Update sort when match status changes

### **CRM Shortcuts**
- [ ] Open field in CRM customization page
- [ ] Open entity in CRM
- [ ] Open example record in CRM
- [ ] Copy CRM URLs to clipboard
