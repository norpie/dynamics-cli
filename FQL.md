# FetchXML Query Language (FQL) Specification

## Overview
FQL is a terse, jq-inspired query language that compiles to FetchXML for Dynamics 365. It prioritizes brevity and readability while maintaining full FetchXML capabilities.

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

### Joins (Link Entities)
```fql
# Simple join
.account | join(.contact on .primarycontactid)

# Join with attributes from joined entity
.account 
  | .name, .revenue
  | join(.contact on .primarycontactid 
    | .firstname, .lastname)

# Multiple joins
.account 
  | join(.contact on .primarycontactid)
  | join(.user on .owninguser)

# Outer join
.account | leftjoin(.contact on .primarycontactid)

# Join with alias and conditions
.account as a
  | join(.contact as c on .primarycontactid 
    | .statecode == 0)

# Join with complex relationship
.account 
  | join(.contact on .accountid -> .parentcustomerid)
```

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
```

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
  | join(.opportunity as o on .accountid 
    | .estimatedvalue > 50000)
  | distinct
```

## CLI Integration Examples

```bash
# Basic query
fql query '.account | .name, .revenue | limit(10)'

# With output format
fql query '.account | limit(5)' --output json

# Save query to file
echo '.account | .revenue > 1000000' > big_accounts.fql
fql query -f big_accounts.fql

# Pipe to other tools
fql query '.contact | .emailaddress1' | grep '@contoso.com'

# Export to CSV
fql query '.opportunity | .name, .estimatedvalue' --output csv > opportunities.csv

# With environment selection
fql query '.account' --env production

# Watch mode (live updates)
fql watch '.case | .statecode == 0 | count()'
```

## Complete Examples

### Find high-value opportunities with account details
```fql
.opportunity as o
  | .estimatedvalue > 100000
  | .statecode == 0
  | join(.account as a on .customerid 
    | .name, .industrycode)
  | join(.user as u on .ownerid 
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
  | leftjoin(.activitypointer as a on .regardingobjectid
    | .activityid)
  | a.activityid == null
  | c.fullname, c.emailaddress1, c.createdon
```

## Parser Implementation Notes (Rust)

The parser should:
1. Tokenize the FQL string
2. Build an AST representing the query structure
3. Transform AST to FetchXML DOM
4. Serialize to FetchXML string

Key structures:
- Entity nodes
- Filter condition trees  
- Join relationships
- Attribute selections
- Aggregation functions
- Order/pagination modifiers

The CLI tool can leverage `clap` for argument parsing and `quick-xml` or similar for FetchXML generation.

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
