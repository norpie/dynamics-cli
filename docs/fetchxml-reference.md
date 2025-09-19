# Microsoft FetchXML Reference

This document provides a comprehensive reference for Microsoft FetchXML based on official Microsoft documentation. Use this as the authoritative source for validating FQL parser output against Microsoft standards.

## Overview

FetchXML is Microsoft's proprietary XML-based query language for retrieving data from Microsoft Dataverse (formerly Common Data Service) and Dynamics 365. It provides a platform-neutral way to express queries that can be used with:

- SDK for .NET `FetchExpression` class
- Dataverse Web API
- Power Apps and Power Automate
- Reports and views

**Key Constraints:**
- FetchXML is read-only (no Create/Update/Delete operations)
- Maximum 5,000 records per query (configurable up to system limits)
- Supports complex joins, filtering, aggregation, and sorting

## Root Element: `<fetch>`

The `<fetch>` element is the root container for all FetchXML queries.

### Required Structure
```xml
<fetch version="1.0" output-format="xml-platform" mapping="logical">
  <entity name="entityname">
    <!-- query content -->
  </entity>
</fetch>
```

### Attributes

| Attribute | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `version` | string | No | "1.0" | FetchXML version |
| `output-format` | string | No | "xml-platform" | Output format specification |
| `mapping` | string | No | "logical" | Attribute name mapping type |
| `distinct` | boolean | No | false | Remove duplicate rows |
| `top` | integer | No | - | Maximum records to return (1-5000) |
| `count` | integer | No | - | Records per page (use with `page`) |
| `page` | integer | No | - | Page number (use with `count`) |
| `returntotalrecordcount` | boolean | No | false | Include total record count in results |
| `aggregate` | boolean | No | false | Enable aggregation mode |
| `aggregatelimit` | integer | No | 50000 | Custom aggregate record limit |
| `no-lock` | boolean | No | false | Legacy attribute (no longer needed) |
| `latematerialize` | boolean | No | false | Performance optimization hint |
| `options` | string | No | - | SQL optimization hints |
| `datasource` | string | No | - | Data retention source specification |
| `useraworderby` | boolean | No | false | Sort choice columns by integer value |

### Attribute Constraints
- `top` cannot be used with `page`, `count`, or `returntotalrecordcount`
- `page` requires `count` to be specified
- `aggregate="true"` is automatically set when aggregation functions are used

## Entity Element: `<entity>`

Defines the primary entity for the query. Only one `<entity>` element is allowed per `<fetch>`.

### Attributes

| Attribute | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `name` | string | Yes | - | Logical name of the entity |
| `alias` | string | No | - | Alias for the entity (used in joins and filters) |

### Example
```xml
<entity name="account" alias="a">
```

## Attribute Selection

### `<attribute>` Element

Specifies individual columns to retrieve from entities.

#### Attributes

| Attribute | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `name` | string | Yes | - | Logical name of the attribute |
| `alias` | string | No | - | Alias for the attribute in results |
| `aggregate` | string | No | - | Aggregation function (count, sum, avg, min, max) |
| `groupby` | boolean | No | false | Use attribute for grouping |
| `dategrouping` | string | No | - | Date grouping (day, week, month, quarter, year) |

#### Examples
```xml
<attribute name="name" />
<attribute name="revenue" alias="total_revenue" />
<attribute name="accountid" aggregate="count" alias="account_count" />
<attribute name="createdon" groupby="true" dategrouping="month" />
```

### `<all-attributes>` Element

Retrieves all non-null column values. **Not recommended** for production use due to performance implications.

```xml
<all-attributes />
```

## Filtering: `<filter>` and `<condition>`

### `<filter>` Element

Groups multiple conditions with logical operators.

#### Attributes

| Attribute | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `type` | string | No | "and" | Logical operator: "and" or "or" |

#### Examples
```xml
<filter type="and">
  <!-- conditions -->
</filter>

<filter type="or">
  <!-- conditions -->
</filter>
```

### `<condition>` Element

Defines individual filter criteria.

#### Attributes

| Attribute | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `attribute` | string | Yes | - | Logical name of the attribute to filter |
| `operator` | string | Yes | - | Filter operator (see operators section) |
| `value` | string | Conditional | - | Single value for comparison |
| `entityname` | string | No | - | Entity name for linked entity conditions |
| `valueof` | string | No | - | Compare to another column value |

#### Examples
```xml
<condition attribute="name" operator="eq" value="Contoso" />
<condition attribute="revenue" operator="gt" value="1000000" />
<condition attribute="parentaccountid" operator="null" />
```

#### Multi-Value Conditions

For operators requiring multiple values (`in`, `between`), use child `<value>` elements:

```xml
<condition attribute="industrycode" operator="in">
  <value>1</value>
  <value>2</value>
  <value>3</value>
</condition>

<condition attribute="createdon" operator="between">
  <value>2023-01-01</value>
  <value>2023-12-31</value>
</condition>
```

## Operators Reference

FetchXML supports a comprehensive set of operators for different data types and comparison needs.

### Comparison Operators

| Operator | Description | Data Types | Value Required | Example |
|----------|-------------|------------|----------------|---------|
| `eq` | Equal to | All | Yes | `<condition attribute="name" operator="eq" value="Contoso" />` |
| `ne` | Not equal to | All | Yes | `<condition attribute="statecode" operator="ne" value="0" />` |
| `gt` | Greater than | Number, Date, String | Yes | `<condition attribute="revenue" operator="gt" value="1000000" />` |
| `ge` | Greater than or equal | Number, Date, String | Yes | `<condition attribute="revenue" operator="ge" value="1000000" />` |
| `lt` | Less than | Number, Date, String | Yes | `<condition attribute="revenue" operator="lt" value="1000000" />` |
| `le` | Less than or equal | Number, Date, String | Yes | `<condition attribute="revenue" operator="le" value="1000000" />` |

### String Operators

| Operator | Description | Wildcards | Example |
|----------|-------------|-----------|---------|
| `like` | Contains or matches pattern | % (any chars), _ (single char) | `<condition attribute="name" operator="like" value="%Contoso%" />` |
| `not-like` | Does not contain or match pattern | % (any chars), _ (single char) | `<condition attribute="name" operator="not-like" value="%Test%" />` |
| `begins-with` | String starts with value | Supports wildcards | `<condition attribute="name" operator="begins-with" value="Con" />` |
| `not-begin-with` | String does not start with value | Supports wildcards | `<condition attribute="name" operator="not-begin-with" value="Test" />` |
| `ends-with` | String ends with value | Supports wildcards | `<condition attribute="name" operator="ends-with" value="Inc" />` |
| `not-end-with` | String does not end with value | Supports wildcards | `<condition attribute="name" operator="not-end-with" value="LLC" />` |

### Collection Operators

| Operator | Description | Value Format | Example |
|----------|-------------|--------------|---------|
| `in` | Value exists in list | Multiple `<value>` elements | See multi-value example above |
| `not-in` | Value does not exist in list | Multiple `<value>` elements | `<condition attribute="industrycode" operator="not-in">` |
| `contain-values` | Choice value is in specified values | Multiple `<value>` elements | For choice/picklist fields |
| `not-contain-values` | Choice value is not in specified values | Multiple `<value>` elements | For choice/picklist fields |

### Range Operators

| Operator | Description | Value Format | Example |
|----------|-------------|--------------|---------|
| `between` | Value is between two values | Two `<value>` elements or comma-separated | See multi-value example above |
| `not-between` | Value is not between two values | Two `<value>` elements or comma-separated | `<condition attribute="revenue" operator="not-between">` |

### Null Operators

| Operator | Description | Value Required | Example |
|----------|-------------|----------------|---------|
| `null` | Value is null/empty | No | `<condition attribute="parentaccountid" operator="null" />` |
| `not-null` | Value is not null/empty | No | `<condition attribute="emailaddress1" operator="not-null" />` |

### Date-Specific Operators

| Operator | Description | Value Format | Example |
|----------|-------------|--------------|---------|
| `on` | Exact date match | Date string | `<condition attribute="createdon" operator="on" value="2023-01-01" />` |
| `not-on` | Not on exact date | Date string | `<condition attribute="createdon" operator="not-on" value="2023-01-01" />` |
| `on-or-before` | On or before date | Date string | `<condition attribute="createdon" operator="on-or-before" value="2023-12-31" />` |
| `on-or-after` | On or after date | Date string | `<condition attribute="createdon" operator="on-or-after" value="2023-01-01" />` |

### Dynamic Date Operators

| Operator | Description | Value Format | Example |
|----------|-------------|--------------|---------|
| `today` | Today's date | No value | `<condition attribute="createdon" operator="today" />` |
| `yesterday` | Yesterday's date | No value | `<condition attribute="createdon" operator="yesterday" />` |
| `tomorrow` | Tomorrow's date | No value | `<condition attribute="createdon" operator="tomorrow" />` |
| `last-x-days` | Last X days | Number | `<condition attribute="createdon" operator="last-x-days" value="30" />` |
| `next-x-days` | Next X days | Number | `<condition attribute="createdon" operator="next-x-days" value="30" />` |
| `this-week` | Current week | No value | `<condition attribute="createdon" operator="this-week" />` |
| `last-week` | Previous week | No value | `<condition attribute="createdon" operator="last-week" />` |
| `next-week` | Next week | No value | `<condition attribute="createdon" operator="next-week" />` |
| `this-month` | Current month | No value | `<condition attribute="createdon" operator="this-month" />` |
| `last-month` | Previous month | No value | `<condition attribute="createdon" operator="last-month" />` |
| `next-month` | Next month | No value | `<condition attribute="createdon" operator="next-month" />` |
| `this-year` | Current year | No value | `<condition attribute="createdon" operator="this-year" />` |
| `last-year` | Previous year | No value | `<condition attribute="createdon" operator="last-year" />` |
| `next-year` | Next year | No value | `<condition attribute="createdon" operator="next-year" />` |

### Fiscal Period Operators

| Operator | Description | Value Format | Example |
|----------|-------------|--------------|---------|
| `in-fiscal-period` | In fiscal period | Period number | `<condition attribute="createdon" operator="in-fiscal-period" value="1" />` |
| `last-fiscal-period` | Last fiscal period | No value | `<condition attribute="createdon" operator="last-fiscal-period" />` |
| `next-fiscal-period` | Next fiscal period | No value | `<condition attribute="createdon" operator="next-fiscal-period" />` |

### Hierarchical Operators

| Operator | Description | Data Types | Example |
|----------|-------------|------------|---------|
| `above` | Records above in hierarchy | Lookup fields | `<condition attribute="parentaccountid" operator="above" valueof="accountid" />` |
| `under` | Records under in hierarchy | Lookup fields | `<condition attribute="parentaccountid" operator="under" valueof="accountid" />` |
| `not-under` | Records not under in hierarchy | Lookup fields | `<condition attribute="parentaccountid" operator="not-under" valueof="accountid" />` |
| `above-or-equal` | Records above or equal in hierarchy | Lookup fields | For hierarchical data relationships |
| `under-or-equal` | Records under or equal in hierarchy | Lookup fields | For hierarchical data relationships |

### Special Value Formats

#### Date Values
- ISO format: `2023-01-01T00:00:00Z`
- Simple format: `2023-01-01`
- Dynamic: `@today-30d`, `@yesterday`, `@this-month`

#### Choice/Picklist Values
- Use integer values for choice fields
- String labels are not supported in conditions

#### Lookup Values
- Use GUID values for lookup fields
- Format: `{12345678-1234-1234-1234-123456789012}`

## Joins: `<link-entity>`

Link-entity elements enable joining related tables to retrieve additional data or apply filters.

### Attributes

| Attribute | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `name` | string | Yes | - | Logical name of the linked entity |
| `alias` | string | No | - | Alias for the linked entity |
| `from` | string | Yes | - | Attribute in the linked entity |
| `to` | string | Yes | - | Attribute in the source entity |
| `link-type` | string | No | "inner" | Join type: "inner" or "outer" |
| `visible` | boolean | No | true | Whether to include linked entity in results |
| `intersect` | boolean | No | false | For many-to-many relationships |

### Join Types

| Type | Description | SQL Equivalent |
|------|-------------|----------------|
| `inner` | Returns only matching records | INNER JOIN |
| `outer` | Returns all records from source, nulls for non-matching | LEFT OUTER JOIN |

### Example
```xml
<link-entity name="contact" alias="c" from="contactid" to="primarycontactid" link-type="inner">
  <attribute name="firstname" />
  <attribute name="lastname" />
  <filter type="and">
    <condition attribute="statecode" operator="eq" value="0" />
  </filter>
</link-entity>
```

### Nested Joins
Link-entities can contain other link-entities for multi-level relationships:

```xml
<link-entity name="account" alias="a" from="accountid" to="customerid">
  <link-entity name="contact" alias="ac" from="contactid" to="primarycontactid">
    <attribute name="fullname" />
  </link-entity>
</link-entity>
```

## Ordering: `<order>`

Specifies how query results should be sorted.

### Attributes

| Attribute | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `attribute` | string | Yes | - | Attribute to sort by |
| `descending` | boolean | No | false | Sort direction (true=DESC, false=ASC) |
| `alias` | string | No | - | Entity alias for the attribute |

### Examples
```xml
<order attribute="name" descending="false" />
<order attribute="revenue" descending="true" />
<order attribute="createdon" descending="true" alias="a" />
```

### Multiple Order Criteria
Multiple `<order>` elements can be specified for secondary sorting:

```xml
<order attribute="industrycode" descending="false" />
<order attribute="revenue" descending="true" />
<order attribute="name" descending="false" />
```

## Aggregation and Grouping

### Aggregation Functions

When using aggregation, set `aggregate="true"` on the `<fetch>` element.

| Function | Description | Example |
|----------|-------------|---------|
| `count` | Count of records | `<attribute name="accountid" aggregate="count" alias="total" />` |
| `sum` | Sum of values | `<attribute name="revenue" aggregate="sum" alias="total_revenue" />` |
| `avg` | Average of values | `<attribute name="revenue" aggregate="avg" alias="avg_revenue" />` |
| `min` | Minimum value | `<attribute name="revenue" aggregate="min" alias="min_revenue" />` |
| `max` | Maximum value | `<attribute name="revenue" aggregate="max" alias="max_revenue" />` |

### Grouping

Use `groupby="true"` on attributes to group results:

```xml
<fetch aggregate="true">
  <entity name="account">
    <attribute name="industrycode" groupby="true" alias="industry" />
    <attribute name="accountid" aggregate="count" alias="count" />
    <attribute name="revenue" aggregate="avg" alias="avg_revenue" />
  </entity>
</fetch>
```

### Date Grouping

Group by date parts using `dategrouping`:

```xml
<attribute name="createdon" groupby="true" dategrouping="month" alias="month" />
```

Valid `dategrouping` values:
- `day`
- `week`
- `month`
- `quarter`
- `year`
- `fiscal-period`
- `fiscal-year`

## Value Element: `<value>`

Used within conditions that require multiple values.

### Usage with `in` operator:
```xml
<condition attribute="industrycode" operator="in">
  <value>1</value>
  <value>2</value>
  <value>3</value>
</condition>
```

### Usage with `between` operator:
```xml
<condition attribute="revenue" operator="between">
  <value>100000</value>
  <value>1000000</value>
</condition>
```

## Constraints and Limitations

### Performance Limits
- Maximum 5,000 records per query by default
- Configurable up to system-defined maximum
- Aggregate queries limited to 50,000 records by default

### Relationship Limits
- Maximum 10 link-entity joins per query
- No circular references allowed
- Join performance degrades with query complexity

### Attribute Limits
- No limit on number of attributes selected
- `<all-attributes>` not recommended for performance
- Linked entity attributes require explicit selection

### Filter Limits
- No explicit limit on number of conditions
- Complex nested filters may impact performance
- Large `in` lists (>1000 values) may cause timeouts

### Date Constraints
- Date values must be valid ISO 8601 format
- Dynamic date operators work in user's time zone
- Fiscal period operators require fiscal year settings

## Validation Rules

### Required Elements
- Every `<fetch>` must contain exactly one `<entity>`
- Every `<condition>` must specify `attribute` and `operator`
- Every `<link-entity>` must specify `name`, `from`, and `to`

### Attribute Naming
- Use logical names, not display names
- Entity and attribute names are case-sensitive
- Aliases must be unique within query scope

### Operator Compatibility
- String operators only work with text fields
- Date operators only work with datetime fields
- Numeric operators only work with number fields
- Hierarchical operators only work with lookup fields

### Value Format Validation
- Dates must be valid ISO format or dynamic expressions
- Numbers must be valid numeric format
- GUIDs must be properly formatted with hyphens and braces
- Boolean values must be "true" or "false" (lowercase)

## Common Patterns

### Filtering with Related Data
```xml
<fetch>
  <entity name="opportunity">
    <attribute name="name" />
    <link-entity name="account" alias="a" from="accountid" to="customerid">
      <filter type="and">
        <condition attribute="industrycode" operator="eq" value="1" />
      </filter>
    </link-entity>
  </entity>
</fetch>
```

### Aggregation with Grouping
```xml
<fetch aggregate="true">
  <entity name="account">
    <attribute name="industrycode" groupby="true" alias="industry" />
    <attribute name="accountid" aggregate="count" alias="total_accounts" />
    <attribute name="revenue" aggregate="sum" alias="total_revenue" />
    <order attribute="total_revenue" descending="true" />
  </entity>
</fetch>
```

### Pagination
```xml
<fetch count="50" page="2" returntotalrecordcount="true">
  <entity name="account">
    <attribute name="name" />
    <order attribute="name" />
  </entity>
</fetch>
```

### Complex Filtering
```xml
<fetch>
  <entity name="account">
    <filter type="and">
      <condition attribute="statecode" operator="eq" value="0" />
      <filter type="or">
        <condition attribute="industrycode" operator="eq" value="1" />
        <condition attribute="revenue" operator="gt" value="1000000" />
      </filter>
    </filter>
  </entity>
</fetch>
```