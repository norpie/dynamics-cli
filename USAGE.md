# Dynamics CLI Usage Guide

## Overview
This guide covers the complete usage of `dynamics-cli`, a command-line tool for interacting with Microsoft Dynamics 365. The CLI features FQL (FetchXML Query Language), a terse, jq-inspired query language that compiles to FetchXML. FQL prioritizes brevity and readability while maintaining full FetchXML capabilities.

**Key Features:**
- **Explicit Join Syntax**: Unambiguous `source.field -> target.field` relationship specification
- **Entity Aliases**: Required for clear field references and join conditions
- **Comprehensive Operators**: Full support for filtering, aggregation, and ordering
- **Date Expressions**: Natural date filtering with `@today-30d` syntax

## Basic Syntax

### Entity Selection
```fql
# Basic entity query
.account

# With attributes (pipe to select)
.account | .name, .accountnumber, .revenue

# All attributes
.account | .*

# With alias
.account as a | .name, .accountnumber
```

### Filtering
```fql
# Simple conditions
.account | .revenue > 1000000

# Multiple conditions (implicit AND)
.account | .revenue > 1000000 | .statecode == 0

# Explicit operators
.account | (.revenue > 1000000 and .statecode == 0)

# OR conditions
.account | (.name ~ "Contoso" or .name ~ "Fabrikam")

# IN operator
.account | .industrycode in [1, 2, 3]

# NULL checks
.account | .parentaccountid != null

# Date filtering
.contact | .createdon >= @today-30d
.contact | .birthday between [@2020-01-01, @2020-12-31]
```

### Operators
- `==` equals
- `!=` not-equal
- `>`, `>=`, `<`, `<=` comparisons
- `~` like (contains)
- `!~` not-like
- `^=` begins-with
- `$=` ends-with
- `in` in list
- `!in` not-in list
- `between` range operator
- `->` join relationship (used in join conditions)

### Joins (Link Entities)

FQL uses **explicit join syntax** to eliminate ambiguity about which entity fields belong to. The syntax is:
`join(.entity as alias on source.field -> target.field)`

```fql
# Simple join - contact.contactid references account.primarycontactid
.account | join(.contact as c on c.contactid -> account.primarycontactid)

# Join with attributes from joined entity
.account
  | .name, .revenue
  | join(.contact as c on c.contactid -> account.primarycontactid
    | .firstname, .lastname)

# Multiple joins
.account
  | join(.contact as c on c.contactid -> account.primarycontactid)
  | join(.user as u on u.systemuserid -> account.owninguser)

# Outer join (left join)
.account | leftjoin(.contact as c on c.contactid -> account.primarycontactid)

# Join with alias and conditions
.account as a
  | join(.contact as c on c.contactid -> a.primarycontactid
    | .statecode == 0)

# Complex relationship - contact.parentcustomerid references account.accountid
.account
  | join(.contact as c on c.parentcustomerid -> account.accountid)

# Activity joins - activity.regardingobjectid references contact.contactid
.contact as c
  | leftjoin(.activitypointer as a on a.regardingobjectid -> c.contactid)

# Opportunity joins - opportunity.customerid references account.accountid
.account as acc
  | join(.opportunity as opp on opp.customerid -> acc.accountid)
```

#### Join Syntax Rules:
- **Always use entity aliases**: Required for explicit field references
- **Source -> Target**: Read as "where source.field equals target.field"
- **Common patterns**:
  - `contact.contactid -> account.primarycontactid` (contact is primary contact)
  - `opportunity.customerid -> account.accountid` (opportunity belongs to account)
  - `contact.parentcustomerid -> account.accountid` (contact belongs to account)
  - `activitypointer.regardingobjectid -> contact.contactid` (activity about contact)

### Aggregations
```fql
# Count
.account | count()

# Count with grouping
.account | group(.industrycode) | count()

# Multiple aggregations
.account 
  | group(.industrycode) 
  | count() as total, avg(.revenue) as avg_rev, sum(.revenue) as total_rev

# Having clause
.account 
  | group(.industrycode) 
  | count() as cnt 
  | having(cnt > 5)
```

### Ordering and Pagination
```fql
# Order by
.account | order(.revenue desc, .name asc)

# Limit (top)
.account | limit(50)

# Paging
.account | page(3, 50)  # page 3, 50 records per page

# Combined
.account
  | .revenue > 1000000
  | order(.revenue desc)
  | limit(10)

# Automatic default limits
.account        # Automatically limited to default (100 records)
.account | .*   # Also automatically limited to default
```

**Note**: All FQL queries automatically include a default result limit (100 records) when no explicit `limit()` is specified. This prevents accidentally retrieving huge result sets. The default can be configured using `dynamics-cli settings set default-query-limit <N>`.

### Advanced Features

#### Fetch Options
```fql
# Distinct
.account | distinct | .industrycode

# No lock
.account | options(nolock: true)

# Return total count
.account | options(returntotalrecordcount: true)
```

#### Related Entity Columns (Formatted Values)
```fql
# Include formatted values and lookup names
.account 
  | .name, .revenue, .ownerid
  | options(formatted: true)
```

#### Complex Expressions
```fql
# Nested conditions
.opportunity
  | (.estimatedvalue > 100000 and
     (.statecode == 0 or .closeprobability > 80))

# Subquery-like patterns using joins
.account as a
  | join(.opportunity as o on o.customerid -> a.accountid
    | .estimatedvalue > 50000)
  | distinct
```

## Query Command Syntax

### Basic Command Structure
```bash
dynamics-cli query [QUERY] [OPTIONS]
```

### Arguments
- `QUERY` - FQL query string (optional if using `--file`)

### Options
- `--file <PATH>` or `-f <PATH>` - Execute FQL query from a file instead of command line
- `--format <FORMAT>` - Output format (default: `json`)
  - `json` - JSON format (default)
  - `xml` - XML format
  - `csv` - CSV format
  - `fetchxml` - Raw FetchXML (for debugging, only with `--dry`)
- `--pretty` or `-p` - Pretty print the output
- `--dry` - Show generated FetchXML without executing the query
- `--output <PATH>` or `-o <PATH>` - Save query results to file
- `--stats` - Show query execution time and statistics

### Usage Notes
- Either provide a query string OR use `--file`, but not both
- The `--dry` flag is useful for debugging query translation to FetchXML
- The `--stats` flag shows parse time, execution time, and total time
- CSV format is ideal for importing into spreadsheets or data analysis tools
- All queries automatically include a default result limit (100 records) unless explicitly overridden with `limit()`

## CLI Integration Examples

```bash
# Basic query execution
dynamics-cli query '.account | .name, .revenue | limit(10)'

# With output format options
dynamics-cli query '.account | limit(5)' --format json
dynamics-cli query '.account | limit(5)' --format xml
dynamics-cli query '.account | limit(5)' --format csv

# Pretty printing
dynamics-cli query '.account | limit(5)' --format json --pretty

# Execute query from file
echo '.account | .revenue > 1000000' > big_accounts.fql
dynamics-cli query --file big_accounts.fql

# Execute query from file with formatting
dynamics-cli query --file big_accounts.fql --format csv --pretty

# Show generated FetchXML without executing (dry run)
dynamics-cli query '.account | limit(5)' --dry
dynamics-cli query '.account | limit(5)' --dry --pretty  # Pretty-printed FetchXML

# Save results to file
dynamics-cli query '.account | limit(100)' --format csv --output accounts.csv
dynamics-cli query --file query.fql --output results.json

# Show execution statistics (timing information)
dynamics-cli query '.account | limit(5)' --stats
dynamics-cli query '.contact | limit(100)' --format json --stats

# Pipe to other tools (using JSON format for processing)
dynamics-cli query '.contact | .emailaddress1' --format json | jq -r '.value[].emailaddress1' | grep '@contoso.com'

# Extract data directly to CSV
dynamics-cli query '.opportunity | .name, .estimatedvalue' --format csv --output opportunities.csv

# Using default result limits (automatically limited to configured default)
dynamics-cli query '.account'  # Returns default limit (100 records)

# Override default limits
dynamics-cli query '.account | limit(500)'  # Returns 500 records

# Complex query with multiple output options
dynamics-cli query '.opportunity as o | .estimatedvalue > 100000 | join(.account as a on a.accountid -> o.customerid | .name) | order(.estimatedvalue desc)' --format csv --pretty

# Authentication and environment management
dynamics-cli auth setup --name production  # Setup authentication for production
dynamics-cli auth select production        # Switch to production environment
dynamics-cli auth status                   # Check current authentication

# Entity mapping management (for custom entities)
dynamics-cli entity add my_custom_entity my_custom_entities  # Add custom entity mapping
dynamics-cli entity list                                     # View all entity mappings

# Settings management
dynamics-cli settings set default-query-limit 50  # Set default limit to 50 records
dynamics-cli settings show                         # View all current settings
dynamics-cli settings reset default-query-limit   # Reset to default (100)
```

## Complete Examples

### Find high-value opportunities with account details
```fql
.opportunity as o
  | .estimatedvalue > 100000
  | .statecode == 0
  | join(.account as a on a.accountid -> o.customerid
    | .name, .industrycode)
  | join(.user as u on u.systemuserid -> o.ownerid
    | .fullname)
  | order(.estimatedvalue desc)
  | limit(20)
```

### Account summary by industry
```fql
.account 
  | .statecode == 0
  | group(.industrycode)
  | count() as total,
    avg(.revenue) as avg_revenue,
    sum(.numberofemployees) as total_employees
  | having(total > 5)
  | order(avg_revenue desc)
```

### Recent contacts without activities
```fql
.contact as c
  | .createdon >= @today-7d
  | leftjoin(.activitypointer as a on a.regardingobjectid -> c.contactid
    | .activityid)
  | a.activityid == null
  | c.fullname, c.emailaddress1, c.createdon
```

## Parser Implementation (Rust)

The FQL parser is implemented with a complete lexer-parser-codegen pipeline:

### Implementation Architecture
1. **Lexer** (`src/fql/lexer.rs`): Tokenizes FQL strings into structured tokens
2. **Parser** (`src/fql/parser.rs`): Builds Abstract Syntax Tree (AST) from tokens using recursive descent
3. **AST** (`src/fql/ast.rs`): Type-safe representation of query structure
4. **XML Generator** (`src/fql/xml.rs`): Converts AST to FetchXML strings

### Key AST Structures
- `Query` - Root query node with entity, attributes, filters, joins, etc.
- `Entity` - Entity selection with optional alias
- `Filter` - Condition trees with logical operators (AND/OR)
- `Join` - Link-entity relationships with join conditions
- `Attribute` - Column selections with optional aliases
- `Aggregation` - Functions like count(), sum(), avg()
- `OrderBy` - Sort specifications with direction

### Integration Features
- **Automatic Default Limits**: XML generator applies configurable default limits
- **Entity Name Resolution**: Integrates with entity mapping system
- **Live Query Execution**: AST compiles to FetchXML and executes against Dynamics 365
- **Multiple Output Formats**: JSON, XML, and tabular presentation

### Testing
Comprehensive test suite in `tests/fql_parser_test.rs` validates the complete pipeline:
- FQL → Tokens → AST → FetchXML transformation
- All FQL language features with expected XML output
- 36+ test cases covering the complete specification

## Comparison

### FetchXML (Traditional)
```xml
<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" top="10">
  <entity name="account">
    <attribute name="name" />
    <attribute name="revenue" />
    <filter type="and">
      <condition attribute="revenue" operator="gt" value="1000000" />
      <condition attribute="statecode" operator="eq" value="0" />
    </filter>
    <order attribute="revenue" descending="true" />
  </entity>
</fetch>
```

### FQL (New)
```fql
.account 
  | .name, .revenue
  | .revenue > 1000000
  | .statecode == 0
  | order(.revenue desc)
  | limit(10)
```
