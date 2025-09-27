# Python's Non-Obvious Transformation Logic

**1. Raad van Bestuur (Board) Date Matching:**
- Takes a date from "Raad van Bestuur" column
- Looks it up in `bestuur_deadlines.csv` to find existing deadline ID
- Supports multiple date formats (`%Y-%m-%d`, `%d/%m/%Y`, etc.)
- Creates a self-referencing deadline relationship

**2. Email Generation from Names:**
- `"Jan Van Der Berg"` → `jvandenberg@vaf.be`
- Handles Dutch name prefixes (`van`, `de`, `der`, `den`, `te`, `ten`)
- Falls back to Excel Email column if available
- Looks up SystemUser ID from `vaf_systemusers.csv`

**3. X-marked Column Detection:**
- Iterates through ALL Excel columns
- For any column with a value (X, checkmark, etc.)
- Checks if column header matches CSV lookup tables:
  - `support_types` → `cgk_support.csv`
  - `categories` → `cgk_category.csv`
  - `lengths` → `cgk_length.csv`
  - `flemish_shares` → `cgk_flemishshare.csv`
- Creates N:N relationship arrays with both names and IDs

**4. Date/Time Combination:**
- Combines `Datum` + `Tijd` into single datetime
- Defaults to `10:00` if time missing
- Handles various date object types (string, datetime, date)

**5. Complex Lookup Resolution:**
- Case-insensitive matching across all lookups
- Pillar matching with parentheses removal
- Fund validation and ID resolution
- Commission name-to-ID mapping

**6. Entity Name Generation:**
- Creates `cgk_name` as: `"Deadline Name - 2024-12-25 14:30"`
- Combines deadline name + formatted datetime

You're right - this is **way more complex** than simple field mapping! The TUI would need to handle all these business rules, which is why hardcoding makes more sense.

**Should I create a Rust equivalent** that implements all these transformation rules?