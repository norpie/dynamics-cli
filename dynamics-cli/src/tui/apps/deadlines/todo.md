# Deadlines Module Rewrite - TODO

**Architecture**: Multi-app flow (like migration module). Each screen = separate App, navigation via `Command::NavigateTo(AppId)`.

## Current App Flow (Actual Implementation)

```
DeadlinesEnvironmentSelectApp (select environment)
    ↓ (passes environment_name)
DeadlinesFileSelectApp (file browser → select file → load sheets → select sheet)
    ↓ (will pass environment_name + file_path + sheet_name)
[Next app TBD - probably field mapping or validation]
```

**Note**: Setup/entity mapping step has been deferred. We're starting with the file selection flow first, then will add entity mapping configuration later.

## Original Planned Flow (For Reference)

```
DeadlinesEnvironmentSelectApp
    ↓
DeadlinesSetupApp (one-time: prefix → entity discovery → mapping)
    ↓
DeadlinesFileSelectApp
    ↓
DeadlinesCacheCheckApp
    ↓
DeadlinesValidationApp
    ↓
DeadlinesTransformApp
    ↓
DeadlinesReviewApp
```

## 1. Core Structure (Must Create First)

**Directory Layout:**
```
tui/apps/deadlines/
├── mod.rs                              # Exports all apps
├── deadlines_environment_select_app.rs # App 1: Environment selection
├── deadlines_setup_app.rs              # App 2: Entity mapping setup
├── deadlines_file_select_app.rs        # App 3: File/sheet browser
├── deadlines_cache_check_app.rs        # App 4: Cache check + refresh
├── deadlines_validation_app.rs         # App 5: Structure validation
├── deadlines_transform_app.rs          # App 6: Data transformation
├── deadlines_review_app.rs             # App 7: Error review
├── models.rs                           # Shared types (params, results)
└── shared/                             # Shared logic across apps
    ├── mod.rs
    ├── cache/
    ├── transformation/
    └── validation/
```

## 2. App Implementations (All New Files)

Each app needs (pattern from migration apps):
- [ ] Struct implementing `App` trait
- [ ] `State` struct with app-specific state
- [ ] `Msg` enum for app messages
- [ ] `update()` function (pure function pattern)
- [ ] `view()` function for rendering

### App 1: Environment Selection ✅ COMPLETE
**File:** `deadlines_environment_select_app.rs`
- ✅ List available environments from config
- ✅ Select environment and navigate to File Select
- ✅ State: list of environments, selected index
- ✅ Passes `FileSelectParams { environment_name }` to next app

### App 2: File Selection ✅ COMPLETE
**File:** `deadlines_file_select_app.rs`
- ✅ File browser widget (custom reusable FileBrowser widget)
- ✅ Filter .xlsx files (and directories)
- ✅ Auto-select first Excel file on directory change
- ✅ Load sheets from selected file (calamine, async)
- ✅ Sheet selector with panel
- ✅ Back button → returns to Environment Select
- ✅ Continue button → proceeds with selected file + sheet
- ✅ State: environment_name, file_browser_state, selected_file, available_sheets (Resource<Vec<String>>), sheet_selector
- ✅ Viewport height tracking for proper scrolling
- ✅ Navigate to: Environment Select (back) or Next App (continue)

**New Widget Created:** `FileBrowser` (reusable)
- ✅ `FileBrowserState` - manages directory navigation, filtering, selection
- ✅ `FileBrowserEntry` - represents file/directory
- ✅ `FileBrowserAction` - FileSelected, DirectoryEntered, DirectoryChanged
- ✅ `FileBrowserEvent` - Navigate, Activate, GoUp, Refresh
- ✅ Custom key handler (treats Enter as navigation, not activation)
- ✅ Virtual scrolling with scrollbar
- ✅ Filter support for custom file type filtering

### App 3: Setup (Entity Mapping) - DEFERRED
**File:** `deadlines_setup_app.rs` (not yet created)
- [ ] Prefix input (text field)
- [ ] Entity discovery (async API call → loading screen)
- [ ] Entity mapping UI (logical types ↔ discovered entities)
- [ ] Validation (async API validation)
- [ ] Save to DeadlineConfig
- [ ] State: prefix, discovered entities, mappings, validation status
- [ ] Navigate to: File Select on success, back to Environment Select on cancel

**Note**: This step has been deferred. We're building the file-to-mapping flow first.

### App 4: Cache Check
**File:** `deadlines_cache_check_app.rs`
- [ ] Check cache freshness on init
- [ ] Show refresh progress modal if stale (entity-by-entity)
- [ ] Parallel entity fetching
- [ ] State: cache status, refresh progress per entity
- [ ] Navigate to: Validation when cache ready

### App 5: Validation
**File:** `deadlines_validation_app.rs`
- [ ] Load Excel data (async)
- [ ] Validate structure (column → entity type matching)
- [ ] Show warnings popup (unmatched columns)
- [ ] "Continue" → Transformation, "Cancel" → File Select
- [ ] State: excel data, validation result, show warnings popup

### App 6: Transformation
**File:** `deadlines_transform_app.rs`
- [ ] Transform Excel rows → Dynamics entities (async)
- [ ] Lookup resolution (fuzzy matching)
- [ ] Junction relationship handling
- [ ] Timezone conversions
- [ ] Progress bar (row X/Y)
- [ ] State: transformation progress, current row
- [ ] Navigate to: Review when complete

### App 7: Review
**File:** `deadlines_review_app.rs`
- [ ] List rows with validation warnings
- [ ] Detail panel for selected row (all warnings + field values)
- [ ] "Proceed" → (future: upload), "Cancel" → File Select
- [ ] State: transformed records, selected row index, show details

## 3. Shared Subsystems (Port from Old Code)

**Located in:** `shared/` subdirectory (used by multiple apps)

### Shared Models (`models.rs`)
- [ ] `EnvironmentParams` - Passed to Setup/FileSelect/etc apps
- [ ] `FileSelectionResult` - File path + sheet name
- [ ] `ValidationResult` - Matched/unmatched columns
- [ ] `TransformedRecord` - Entity data + warnings
- [ ] `CacheStatus`, `CacheProgress` - Cache state types

### Cache Subsystem (`shared/cache/`)
**Port from:** `commands/deadlines/csv_cache.rs` (21KB)
- [ ] `cache/mod.rs` - CacheManager struct, freshness checks
- [ ] `cache/fetch.rs` - Parallel entity fetching logic
- [ ] Functions: `check_freshness()`, `refresh()`, `load_entity_cache()`

### Transformation Subsystem (`shared/transformation/`)
**Port from:** `commands/deadlines/data_transformer.rs` (31KB)
- [ ] `transformation/mod.rs` - DataTransformer struct, main API
- [ ] `transformation/lookup.rs` - Fuzzy matching (Levenshtein)
- [ ] `transformation/junction.rs` - Many-to-many relationship handling
- [ ] `transformation/timezone.rs` - Brussels timezone conversions
- [ ] Function: `transform_sheet_data(&SheetData) -> Vec<TransformedRecord>`

### Validation Subsystem (`shared/validation/`)
**Port from:** `commands/deadlines/validation.rs` (8KB)
- [ ] `validation/mod.rs` - Validation orchestrator
- [ ] `validation/structure.rs` - Excel column validation
- [ ] `validation/warnings.rs` - Per-row warning generation
- [ ] Functions: `validate_excel_structure()`, `generate_warnings()`

### Config (`shared/config.rs`)
**Port from:** `commands/deadlines/config.rs` (9KB)
- [ ] `DeadlineConfig` struct (entity mappings per environment)
- [ ] `EnvironmentConfig` struct (prefix, entity map)
- [ ] Load/save functions for SQLite persistence

### Entity Discovery (`shared/discovery.rs`)
**Port from:** `commands/deadlines/entity_discovery.rs` (12KB)
- [ ] `discover_entities(prefix)` - API call to fetch entities
- [ ] `validate_entity_mappings()` - Verify entities exist
- [ ] `DiscoveredEntity` struct

### Excel Parser (`shared/excel.rs`)
**Port from:** `commands/deadlines/excel_parser.rs` (2KB)
- [ ] `parse_excel_file(path, sheet)` - Load Excel data
- [ ] `SheetData` struct (rows + columns)

## 4. Reusable Components

**New Widgets Created:**
- ✅ **FileBrowser** (`tui/widgets/file_browser.rs`) - Reusable file/directory browser
  - Supports custom filtering
  - Auto-selection on directory change
  - Virtual scrolling with ListState
  - Enter key treated as navigation (not activation)

**From `tui/widgets/` (Already Exist):**
- ✅ TextInputField (prefix input in Setup)
- ✅ List (environment list, error list) - **Enhanced with viewport height tracking**
- ✅ SelectField (sheet selection)
- ✅ AutocompleteField (entity search in Setup)
- ✅ Scrollable (long lists)

**From `tui/apps/screens/`:**
- ✅ LoadingScreen (async operations - entity discovery, validation, transformation)
- ✅ ErrorScreen (error handling)

**From migration apps:**
- ✅ Modal patterns (confirmation dialogs, forms)
- ✅ List navigation (ListState)
- ✅ Form validation (Validate macro)

## 5. Integration (Add to TUI Runtime)

### AppId Enum (`tui/command.rs`)
- ✅ Add `DeadlinesEnvironmentSelect` variant
- ✅ Add `DeadlinesFileSelect` variant
- [ ] Add `DeadlinesSetup` variant (deferred)
- [ ] Add `DeadlinesCacheCheck` variant
- [ ] Add `DeadlinesValidation` variant
- [ ] Add `DeadlinesTransform` variant
- [ ] Add `DeadlinesReview` variant

### Module Export (`tui/apps/mod.rs`)
- ✅ Add `pub mod deadlines;`
- ✅ Export app structs + state types

### Runtime Registration (`tui/multi_runtime.rs`)
- ✅ Register DeadlinesEnvironmentSelectApp
- ✅ Register DeadlinesFileSelectApp
- [ ] Register remaining apps as they're built

### App Launcher
- ✅ Add "Deadlines" option to launcher menu
- ✅ Entry point → DeadlinesEnvironmentSelectApp

### Models (`models.rs`)
- ✅ `FileSelectParams` - Passes environment_name to file select app

---

## Implementation Priority

**Phase 1: Foundation + First Apps** ✅ COMPLETE
1. ✅ `models.rs` - Initial types (`FileSelectParams`)
2. ✅ **FileBrowser widget** - Reusable file browser with filtering
3. ✅ **List widget enhancement** - Viewport height tracking for proper scrolling
4. ✅ `deadlines_environment_select_app.rs` - Environment selection
5. ✅ `deadlines_file_select_app.rs` - File browser + sheet selector with buttons
6. ✅ Integration - AppIds, runtime registration, launcher entry

**Phase 2: Next App** (Current Work)
7. [ ] Determine next step: Field mapping or validation?
8. [ ] Create corresponding app based on decision

**Phase 3: Shared Logic** (As Needed)
- [ ] `shared/config.rs` - Port DeadlineConfig + persistence (when needed for Setup app)
- [ ] `shared/excel.rs` - Port Excel parsing (when needed for validation/transformation)
- [ ] `shared/discovery.rs` - Port entity discovery (when needed for Setup app)
- [ ] `shared/validation/` - Port validation logic (when validation app is built)
- [ ] `shared/transformation/` - Port transformation logic (when transform app is built)
- [ ] `shared/cache/` - Port cache management (when cache check app is built)

**Phase 4: Remaining Apps** (Future)
- [ ] Field mapping app (or integrate into existing flow)
- [ ] `deadlines_cache_check_app.rs` - Progress tracking
- [ ] `deadlines_validation_app.rs` - Validation + modal
- [ ] `deadlines_transform_app.rs` - Heavy async processing
- [ ] `deadlines_review_app.rs` - Review UI + details panel
- [ ] `deadlines_setup_app.rs` - Entity mapping setup (optional/admin feature)

**Phase 5: Cleanup**
- [ ] Delete old `commands/deadlines/` directory (5545 lines!)
- [ ] Update documentation

---

## Key Differences from Old Code

**Old (commands/deadlines):**
- ❌ Manual terminal management per phase
- ❌ Monolithic event loops
- ❌ Manual loading flags (`is_loading: bool`)
- ❌ Custom modal implementations per screen
- ❌ No back navigation
- ❌ Sequential phase progression only

**New (tui/apps/deadlines):**
- ✅ Multi-app architecture (each screen = App)
- ✅ `Resource<T>` for async state (Loading | Loaded | Error)
- ✅ Reusable widgets + LoadingScreen
- ✅ Back navigation via `Command::NavigateTo()`
- ✅ Non-linear flow (can jump between apps)
- ✅ Testable business logic in `shared/`
- ✅ Consistent patterns across all apps

---

## Testing Strategy

**Unit tests** (in `shared/` modules):
- [ ] Fuzzy matching algorithm (transformation/lookup.rs)
- [ ] Timezone conversions (transformation/timezone.rs)
- [ ] Excel validation logic (validation/structure.rs)
- [ ] Warning generation (validation/warnings.rs)

**Manual TUI testing:**
```bash
cargo run -- deadlines
RUST_LOG=debug cargo run -- deadlines  # With logs → dynamics-cli.log
```

**Debugging workflow:**
1. Run with `RUST_LOG=debug`
2. Reproduce issue in TUI
3. Exit app
4. Read `dynamics-cli.log`
5. Add more logging if needed

---

## Current Status Summary

### ✅ Completed (Phase 1)

**Apps:**
1. **DeadlinesEnvironmentSelectApp** - Lists environments, selects one, navigates to file select
2. **DeadlinesFileSelectApp** - Complete file + sheet selection with:
   - FileBrowser widget for navigating directories
   - Excel file filtering (.xlsx, .xls, .xlsm)
   - Auto-selection of first Excel file
   - Async sheet loading with calamine
   - Sheet selector dropdown
   - Back/Continue buttons
   - Proper scrolling with viewport height tracking

**New Widgets:**
- **FileBrowser** - Fully reusable file/directory browser
  - Custom filtering support
  - Virtual scrolling
  - Enter key as navigation
  - Auto-selection helpers

**Enhancements:**
- **List widget** - Added `on_render` callback and viewport height tracking
- **ListState** - Now tracks viewport height for proper scroll calculations

**Integration:**
- AppIds added to `tui/command.rs`
- Apps registered in `tui/multi_runtime.rs`
- Launcher menu entry

### 🔄 Next Steps

**Immediate:** Decide on next app in flow:
- Option A: Field mapping (map Excel columns to Dynamics fields)
- Option B: Validation (check Excel structure)
- Option C: Something else based on requirements

**Future:** Build remaining apps as needed (cache, transform, review)
