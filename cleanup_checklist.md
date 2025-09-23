# Cleanup Application - Complete Feature Checklist

## 🏗️ Core Architecture
- [ ] Multi-module architecture (cleanup, dynamics_api, tui_toolkit, deadlines)
- [ ] OAuth2 authentication with token caching
- [ ] Batch processing with pause/resume capability
- [ ] Comprehensive error handling and logging system
- [ ] Configuration management via .env files

## 🖥️ Terminal User Interface (TUI)
- [ ] Interactive menu system with keyboard navigation
- [ ] Six main sections: Caching, Fetching, Processing, Inspecting, Pushing, System
- [ ] Real-time streaming display with scrolling
- [ ] Progress bars with tqdm integration
- [ ] Global pause/resume functionality
- [ ] Table rendering with pagination
- [ ] Formatted error display and debugging

## 📊 Data Management & I/O
- [ ] CSV import/export (latin-1, utf-8-sig encoding)
- [ ] Excel-compatible CSV export format
- [ ] JSON dumping for debugging operations
- [ ] Pickle-based caching system with metadata
- [ ] Efficient dictionary-based data indexing
- [ ] Multiple data source handling (contacts, CGK, connections)

## 🔗 Microsoft Dynamics 365 API Integration
- [ ] OAuth2 authentication with automatic token refresh
- [ ] Multipart batch requests for bulk operations
- [ ] FetchXML query generation and execution
- [ ] Rate limiting and retry logic
- [ ] Entity fetching with filtering and pagination
- [ ] Bulk field updates with conflict resolution
- [ ] Contact merge operations with field priority rules
- [ ] Notes management and attachment handling
- [ ] Connection relationship processing

## 🧹 Data Normalization & Validation
### Contact Fields
- [ ] Name normalization (diacritics, case, initials)
- [ ] Email validation (RFC compliance, TLD verification)
- [ ] Phone number international format conversion (E.164)
- [ ] Address component extraction and standardization
- [ ] Date processing with age restrictions (18+)

### Validation Rules  
- [ ] Email format and deliverability checking
- [ ] Country-specific phone validation (Belgium focus)
- [ ] Name capitalization and diacritic handling
- [ ] Birth date validation with placeholder detection
- [ ] Address component matching and hierarchy validation

## 🔍 Contact Matching & Deduplication
- [ ] Multi-field matching strategies (Email+Name, Phone+Name, Name+Address, Birthday+Name)
- [ ] Graph-based transitive relationship detection
- [ ] Connected components algorithm for grouping
- [ ] Normalized comparison with diacritic handling
- [ ] Efficient lookup structure building
- [ ] Match reason tracking and audit trail
- [ ] Fuzzy address component comparison

## 🔀 Contact Merging Logic
### Field Merge Rules
- [ ] Name merging (priority to diacritics, capitalization, frequency)
- [ ] Email selection (recency-based, creation date priority)
- [ ] Phone number consolidation (most common with newest fallback)
- [ ] Address merging (component-wise with hierarchical fallback)
- [ ] Date merging (timezone correction, placeholder avoidance)
- [ ] Gender merging (Male/Female priority over Other)
- [ ] Website handling (all-or-nothing matching)

### Special Field Handling
- [ ] CGK Azure email pattern matching (@vaf.be)
- [ ] Creator ID OData binding format
- [ ] Parent customer conflict detection and resolution
- [ ] Composite to individual field conversion

## 📦 Batch Processing System
- [ ] N-way to pairwise merge conversion
- [ ] Conflict avoidance (no contact appears twice per batch)
- [ ] Greedy packing algorithm for optimal batch filling
- [ ] Configurable batch sizes (default 10)
- [ ] Progress persistence for pause/resume
- [ ] Error recovery and isolation
- [ ] Real-time progress tracking with ETA
- [ ] Success metrics and performance statistics

## 🗂️ Caching & Performance
- [ ] Named cache system with pickle serialization
- [ ] Cache metadata tracking (timestamps, sizes)
- [ ] Bulk operation result caching
- [ ] Cache management operations (list, delete, clear)
- [ ] Data indexing for fast access
- [ ] Lazy loading of data
- [ ] Connection analysis result caching

## 🔌 Connection & Relationship Processing
- [ ] Direct contact-to-contact relationship analysis
- [ ] CGK contact type mapping and resolution
- [ ] User-to-contact creator tracking
- [ ] Multiple connection source handling (coaching, commission, deadline, payment)
- [ ] Connection deduplication (AuthUser + Role + Target)
- [ ] has_connection flag tracking for all contacts
- [ ] Source-based contact whitelist filtering
- [ ] Generic connection file format auto-detection
- [ ] Duplicate removal strategies (keep newest/oldest)

## 🧪 Testing & Quality Assurance
### Test Coverage
- [ ] Email validation testing (RFC compliance, TLD validation, normalization)
- [ ] Phone validation testing (international formats, country-specific rules)
- [ ] Parent customer ID conflict detection testing
- [ ] Logging utilities comprehensive testing
- [ ] Connection cleanup relationship validation testing

### Testing Patterns
- [ ] Parametrized tests for multiple scenarios
- [ ] Edge case handling (invalid inputs, boundaries)
- [ ] Return value validation (None for invalid, normalized for valid)
- [ ] End-to-end workflow integration testing

## 📋 Inspection & Reporting
### Data Analysis Views
- [ ] Parent customer ID conflict reporting with CSV export
- [ ] Connection statistics with duplicate detection analysis
- [ ] Contact statistics and data quality metrics
- [ ] Match reason detailed audit trail
- [ ] Comprehensive error reporting and debugging

### Export Capabilities
- [ ] Timestamped CSV exports with Excel compatibility
- [ ] JSON debug dumps of operation details
- [ ] Raw API response logging for debugging
- [ ] Structured conflict report generation

## 🔧 Configuration & Environment
- [ ] Environment variable management via dotenv
- [ ] OAuth credential management (client ID, secret, tenant)
- [ ] API configuration (host, version, timeout settings)
- [ ] Cache directory and size management
- [ ] Logging configuration (level, format, file rotation)

## ⚠️ Error Handling & Logging
### Comprehensive Logging
- [ ] Batch-specific operation log files
- [ ] Structured error message formatting
- [ ] Raw request/response debugging logs
- [ ] Performance metrics and timing tracking
- [ ] Complete operation history audit trail

### Error Management
- [ ] Exception handling with graceful degradation
- [ ] HTTP status code interpretation for API errors
- [ ] Detailed validation failure reporting
- [ ] Retry logic and fallback strategies
- [ ] Recovery mechanisms for failed operations

## 🔄 Advanced Features
### Pause/Resume System
- [ ] Global application-wide pause functionality
- [ ] Operation state preservation and caching
- [ ] Resume from last successful operation
- [ ] Interactive keyboard-driven pause/resume control

### VAF-Specific Features
- [ ] @vaf.be email pattern handling
- [ ] Creator tracking and audit trail
- [ ] Connection source-based whitelist filtering
- [ ] Integration with deadlines application (separate)

## 📱 Command Line Interface
- [ ] `python -m cleanup` - Main interactive TUI
- [ ] `pytest` - Complete test suite execution
- [ ] Version and configuration check commands
- [ ] Non-interactive testing capabilities

## 🔄 Integration Points
- [ ] Shared .env configuration with deadlines app
- [ ] dynamics_api library integration
- [ ] tui_toolkit framework utilization
- [ ] Cross-application data format compatibility