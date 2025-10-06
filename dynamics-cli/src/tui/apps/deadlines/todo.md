# Deadlines Module Rewrite - TODO

**Architecture**: Multi-app flow (like migration module). Each screen = separate App, navigation via `Command::NavigateTo(AppId)`.

## App Flow (7 Separate Apps)

```
DeadlinesEnvironmentSelectApp (select environment)
    ↓
DeadlinesSetupApp (one-time: prefix → entity discovery → mapping)
    ↓
DeadlinesFileSelectApp (file browser → sheet selector)
    ↓
DeadlinesCacheCheckApp (cache freshness check + refresh progress)
    ↓
DeadlinesValidationApp (structure validation + warnings popup)
    ↓
DeadlinesTransformApp (data transformation + progress)
    ↓
DeadlinesReviewApp (validation errors review)
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

### App 1: Environment Selection
**File:** `deadlines_environment_select_app.rs`
- [ ] List available environments from config
- [ ] "Create New" option → launches setup app
- [ ] Navigate to setup app (if not configured) or file select (if configured)
- [ ] State: list of environments, selected index

### App 2: Setup (Entity Mapping)
**File:** `deadlines_setup_app.rs`
- [ ] Prefix input (text field)
- [ ] Entity discovery (async API call → loading screen)
- [ ] Entity mapping UI (logical types ↔ discovered entities)
- [ ] Validation (async API validation)
- [ ] Save to DeadlineConfig
- [ ] State: prefix, discovered entities, mappings, validation status
- [ ] Navigate to: File Select on success, back to Environment Select on cancel

### App 3: File Selection
**File:** `deadlines_file_select_app.rs`
- [ ] Directory browser (reuse Tree widget)
- [ ] Filter .xlsx files
- [ ] Load sheets from selected file (calamine)
- [ ] Sheet selector list
- [ ] State: current path, dir entries, selected file, available sheets
- [ ] Navigate to: Cache Check on sheet selection

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

## 4. Reusable Components (Already Exist)

**From `tui/widgets/`:**
- ✅ TextInputField (prefix input in Setup)
- ✅ List (environment list, file browser, error list)
- ✅ SelectField (sheet selection)
- ✅ AutocompleteField (entity search in Setup)
- ✅ Tree (directory navigation in FileSelect)
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
- [ ] Add `DeadlinesEnvironmentSelect` variant
- [ ] Add `DeadlinesSetup` variant
- [ ] Add `DeadlinesFileSelect` variant
- [ ] Add `DeadlinesCacheCheck` variant
- [ ] Add `DeadlinesValidation` variant
- [ ] Add `DeadlinesTransform` variant
- [ ] Add `DeadlinesReview` variant

### Module Export (`tui/apps/mod.rs`)
- [ ] Add `pub mod deadlines;`
- [ ] Export app structs + state types

### Runtime Registration (`tui/multi_runtime.rs`)
- [ ] Register all 7 apps in `create_app_instance()` match
- [ ] Wire up navigation flow

### App Launcher
- [ ] Add "Deadlines" option to launcher menu
- [ ] Entry point → DeadlinesEnvironmentSelectApp

---

## Implementation Priority

**Phase 1: Shared Foundation** (no UI dependencies)
1. [ ] `models.rs` - Core types (params, results)
2. [ ] `shared/config.rs` - Port DeadlineConfig + persistence
3. [ ] `shared/excel.rs` - Port Excel parsing
4. [ ] `shared/discovery.rs` - Port entity discovery
5. [ ] `shared/validation/` - Port validation logic
6. [ ] `shared/transformation/` - Port transformation logic
7. [ ] `shared/cache/` - Port cache management

**Phase 2: Simple Apps First** (learn patterns)
8. [ ] `deadlines_environment_select_app.rs` - List + navigation (like MigrationEnvironmentApp)
9. [ ] `deadlines_file_select_app.rs` - File browser + sheet selector

**Phase 3: Complex Apps** (async operations)
10. [ ] `deadlines_setup_app.rs` - Multi-step workflow + API calls
11. [ ] `deadlines_cache_check_app.rs` - Progress tracking
12. [ ] `deadlines_validation_app.rs` - Validation + modal
13. [ ] `deadlines_transform_app.rs` - Heavy async processing
14. [ ] `deadlines_review_app.rs` - Review UI + details panel

**Phase 4: Integration**
15. [ ] Add AppIds to command.rs
16. [ ] Register apps in runtime
17. [ ] Add to launcher
18. [ ] Manual testing + debug

**Phase 5: Cleanup**
19. [ ] Delete old `commands/deadlines/` directory (5545 lines!)
20. [ ] Update documentation

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

## Start Here

**Recommended first task:** Build shared foundation (Phase 1) before any apps.
- Start with `models.rs` (define all param/result types)
- Then `shared/config.rs` (port DeadlineConfig - needed by all apps)
- Then `shared/excel.rs` (simplest logic, no dependencies)
