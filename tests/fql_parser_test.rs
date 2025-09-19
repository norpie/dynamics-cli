use anyhow::Result;
use dynamics_cli::fql::{parse, to_fetchxml, tokenize};

/// Normalize XML for comparison by removing extra whitespace and newlines
fn normalize_xml(xml: &str) -> String {
    xml.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join("")
        .replace("> <", "><")
        .replace(" />", "/>")
}

/// Test helper to run the complete FQL parsing pipeline
fn test_fql_to_xml(fql: &str, expected_xml: &str) -> Result<()> {
    println!("Testing FQL: {}", fql);

    // Step 1: Tokenize
    let tokens = tokenize(fql)?;
    println!("Tokens: {:?}", tokens.iter().map(|t| &t.token).collect::<Vec<_>>());

    // Step 2: Parse to AST
    let ast = parse(tokens, fql)?;
    println!("AST: {:?}", ast);

    // Step 3: Generate XML
    let xml = to_fetchxml(ast)?;
    println!("Generated XML:\n{}", xml);
    println!("Expected XML:\n{}", expected_xml);

    // Compare XML with normalized whitespace
    let normalized_generated = normalize_xml(&xml);
    let normalized_expected = normalize_xml(expected_xml);

    if normalized_generated != normalized_expected {
        eprintln!("XML mismatch!");
        eprintln!("Generated (normalized): {}", normalized_generated);
        eprintln!("Expected (normalized):  {}", normalized_expected);
        return Err(anyhow::anyhow!("Generated XML does not match expected XML"));
    }

    Ok(())
}

#[test]
fn test_basic_entity_query() {
    let fql = ".account";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_entity_with_attributes() {
    let fql = ".account | .name, .accountnumber, .revenue";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <attribute name="name" />
    <attribute name="accountnumber" />
    <attribute name="revenue" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_entity_with_all_attributes() {
    let fql = ".account | .*";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <all-attributes />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_entity_with_alias() {
    let fql = ".account as a | .name, .accountnumber";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account" alias="a">
    <attribute name="name" />
    <attribute name="accountnumber" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_simple_filter() {
    let fql = ".account | .revenue > 1000000";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <filter type="and">
      <condition attribute="revenue" operator="gt" value="1000000" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_multiple_filters_implicit_and() {
    let fql = ".account | .revenue > 1000000 | .statecode == 0";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <filter type="and">
      <condition attribute="revenue" operator="gt" value="1000000" />
      <condition attribute="statecode" operator="eq" value="0" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_explicit_and_conditions() {
    let fql = ".account | (.revenue > 1000000 and .statecode == 0)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <filter type="and">
      <condition attribute="revenue" operator="gt" value="1000000" />
      <condition attribute="statecode" operator="eq" value="0" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_or_conditions() {
    let fql = ".account | (.name ~ \"Contoso\" or .name ~ \"Fabrikam\")";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <filter type="or">
      <condition attribute="name" operator="like" value="%Contoso%" />
      <condition attribute="name" operator="like" value="%Fabrikam%" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_in_operator() {
    let fql = ".account | .industrycode in [1, 2, 3]";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <filter type="and">
      <condition attribute="industrycode" operator="in">
        <value>1</value>
        <value>2</value>
        <value>3</value>
      </condition>
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_null_check() {
    let fql = ".account | .parentaccountid != null";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <filter type="and">
      <condition attribute="parentaccountid" operator="not-null" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_date_filtering() {
    let fql = ".contact | .createdon >= @today-30d";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="contact">
    <filter type="and">
      <condition attribute="createdon" operator="on-or-after" value="@today-30d" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_between_operator() {
    let fql = ".contact | .birthday between [@2020-01-01, @2020-12-31]";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="contact">
    <filter type="and">
      <condition attribute="birthday" operator="between">
        <value>2020-01-01</value>
        <value>2020-12-31</value>
      </condition>
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_simple_join() {
    let fql = ".account | join(.contact as c on c.contactid -> account.primarycontactid)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <link-entity name="contact" alias="c" from="contactid" to="primarycontactid" link-type="inner">
    </link-entity>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_join_with_attributes() {
    let fql = ".account | .name, .revenue | join(.contact as c on c.contactid -> account.primarycontactid | .firstname, .lastname)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <attribute name="name" />
    <attribute name="revenue" />
    <link-entity name="contact" alias="c" from="contactid" to="primarycontactid" link-type="inner">
      <attribute name="firstname" />
      <attribute name="lastname" />
    </link-entity>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_multiple_joins() {
    let fql = ".account | join(.contact as c on c.contactid -> account.primarycontactid) | join(.user as u on u.systemuserid -> account.owninguser)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <link-entity name="contact" alias="c" from="contactid" to="primarycontactid" link-type="inner">
    </link-entity>
    <link-entity name="user" alias="u" from="systemuserid" to="owninguser" link-type="inner">
    </link-entity>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_left_join() {
    let fql = ".account | leftjoin(.contact as c on c.contactid -> account.primarycontactid)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <link-entity name="contact" alias="c" from="contactid" to="primarycontactid" link-type="outer">
    </link-entity>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_join_with_alias_and_conditions() {
    let fql = ".account as a | join(.contact as c on c.contactid -> a.primarycontactid | .statecode == 0)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account" alias="a">
    <link-entity name="contact" alias="c" from="contactid" to="primarycontactid" link-type="inner">
      <filter type="and">
        <condition attribute="statecode" operator="eq" value="0" />
      </filter>
    </link-entity>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_join_with_complex_relationship() {
    let fql = ".account | join(.contact as c on c.parentcustomerid -> account.accountid)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <link-entity name="contact" alias="c" from="parentcustomerid" to="accountid" link-type="inner">
    </link-entity>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_count_aggregation() {
    let fql = ".account | count()";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" aggregate="true">
  <entity name="account">
    <attribute name="accountid" aggregate="count" alias="count" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_count_with_grouping() {
    let fql = ".account | group(.industrycode) | count()";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" aggregate="true">
  <entity name="account">
    <attribute name="industrycode" groupby="true" alias="industrycode" />
    <attribute name="accountid" aggregate="count" alias="count" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_multiple_aggregations() {
    let fql = ".account | group(.industrycode) | count() as total, avg(.revenue) as avg_rev, sum(.revenue) as total_rev";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" aggregate="true">
  <entity name="account">
    <attribute name="industrycode" groupby="true" alias="industrycode" />
    <attribute name="accountid" aggregate="count" alias="total" />
    <attribute name="revenue" aggregate="avg" alias="avg_rev" />
    <attribute name="revenue" aggregate="sum" alias="total_rev" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_having_clause() {
    let fql = ".account | group(.industrycode) | count() as cnt | having(cnt > 5)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" aggregate="true">
  <entity name="account">
    <attribute name="industrycode" groupby="true" alias="industrycode" />
    <attribute name="accountid" aggregate="count" alias="cnt" />
    <filter type="and">
      <condition attribute="cnt" operator="gt" value="5" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_order_by() {
    let fql = ".account | order(.revenue desc, .name asc)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <order attribute="revenue" descending="true" />
    <order attribute="name" descending="false" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_limit() {
    let fql = ".account | limit(50)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" top="50">
  <entity name="account">
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_page() {
    let fql = ".account | page(3, 50)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" page="3" count="50">
  <entity name="account">
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_combined_query() {
    let fql = ".account | .revenue > 1000000 | order(.revenue desc) | limit(10)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" top="10">
  <entity name="account">
    <filter type="and">
      <condition attribute="revenue" operator="gt" value="1000000" />
    </filter>
    <order attribute="revenue" descending="true" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_distinct() {
    let fql = ".account | distinct | .industrycode";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="true">
  <entity name="account">
    <attribute name="industrycode" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_options_no_lock() {
    let fql = ".account | options(nolock: true)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" no-lock="true">
  <entity name="account">
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_options_return_total_count() {
    let fql = ".account | options(returntotalrecordcount: true)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" returntotalrecordcount="true">
  <entity name="account">
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_options_formatted() {
    let fql = ".account | .name, .revenue, .ownerid | options(formatted: true)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <attribute name="name" />
    <attribute name="revenue" />
    <attribute name="ownerid" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_nested_conditions() {
    let fql =
        ".opportunity | (.estimatedvalue > 100000 and (.statecode == 0 or .closeprobability > 80))";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="opportunity">
    <filter type="and">
      <condition attribute="estimatedvalue" operator="gt" value="100000" />
      <filter type="or">
        <condition attribute="statecode" operator="eq" value="0" />
        <condition attribute="closeprobability" operator="gt" value="80" />
      </filter>
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_subquery_like_patterns() {
    let fql = ".account as a | join(.opportunity as o on o.customerid -> a.accountid | .estimatedvalue > 50000) | distinct";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="true">
  <entity name="account" alias="a">
    <link-entity name="opportunity" alias="o" from="customerid" to="accountid" link-type="inner">
      <filter type="and">
        <condition attribute="estimatedvalue" operator="gt" value="50000" />
      </filter>
    </link-entity>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_high_value_opportunities_example() {
    let fql = r#".opportunity as o
  | .estimatedvalue > 100000
  | .statecode == 0
  | join(.account as a on a.accountid -> o.customerid
    | .name, .industrycode)
  | join(.user as u on u.systemuserid -> o.ownerid
    | .fullname)
  | order(.estimatedvalue desc)
  | limit(20)"#;

    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" top="20">
  <entity name="opportunity" alias="o">
    <filter type="and">
      <condition attribute="estimatedvalue" operator="gt" value="100000" />
      <condition attribute="statecode" operator="eq" value="0" />
    </filter>
    <link-entity name="account" alias="a" from="accountid" to="customerid" link-type="inner">
      <attribute name="name" />
      <attribute name="industrycode" />
    </link-entity>
    <link-entity name="user" alias="u" from="systemuserid" to="ownerid" link-type="inner">
      <attribute name="fullname" />
    </link-entity>
    <order attribute="estimatedvalue" descending="true" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_account_summary_by_industry_example() {
    let fql = r#".account
  | .statecode == 0
  | group(.industrycode)
  | count() as total,
    avg(.revenue) as avg_revenue,
    sum(.numberofemployees) as total_employees
  | having(total > 5)
  | order(avg_revenue desc)"#;

    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" aggregate="true">
  <entity name="account">
    <attribute name="industrycode" groupby="true" alias="industrycode" />
    <attribute name="accountid" aggregate="count" alias="total" />
    <attribute name="revenue" aggregate="avg" alias="avg_revenue" />
    <attribute name="numberofemployees" aggregate="sum" alias="total_employees" />
    <filter type="and">
      <condition attribute="statecode" operator="eq" value="0" />
    </filter>
    <filter type="and">
      <condition attribute="total" operator="gt" value="5" />
    </filter>
    <order attribute="avg_revenue" descending="true" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_recent_contacts_without_activities_example() {
    let fql = r#".contact as c
  | .createdon >= @today-7d
  | leftjoin(.activitypointer as a on a.regardingobjectid -> c.contactid
    | .activityid)
  | a.activityid == null
  | c.fullname, c.emailaddress1, c.createdon"#;

    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="contact" alias="c">
    <attribute name="fullname" />
    <attribute name="emailaddress1" />
    <attribute name="createdon" />
    <filter type="and">
      <condition attribute="createdon" operator="on-or-after" value="@today-7d" />
    </filter>
    <link-entity name="activitypointer" alias="a" from="regardingobjectid" to="contactid" link-type="outer">
      <attribute name="activityid" />
      <filter type="and">
        <condition attribute="activityid" operator="null" />
      </filter>
    </link-entity>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_comparison_example() {
    let fql = r#".account
  | .name, .revenue
  | .revenue > 1000000
  | .statecode == 0
  | order(.revenue desc)
  | limit(10)"#;

    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" top="10">
  <entity name="account">
    <attribute name="name" />
    <attribute name="revenue" />
    <filter type="and">
      <condition attribute="revenue" operator="gt" value="1000000" />
      <condition attribute="statecode" operator="eq" value="0" />
    </filter>
    <order attribute="revenue" descending="true" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_null_condition() {
    let fql = ".account | .primarycontactid == null";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <filter type="and">
      <condition attribute="primarycontactid" operator="null" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_not_null_condition() {
    let fql = ".contact | .emailaddress1 != null";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="contact">
    <filter type="and">
      <condition attribute="emailaddress1" operator="not-null" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_between_condition() {
    let fql = ".opportunity | .estimatedvalue between 10000 and 50000";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="opportunity">
    <filter type="and">
      <condition attribute="estimatedvalue" operator="between" value="10000,50000" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_between_with_dates() {
    let fql = ".account | .createdon between @2023-01-01 and @2023-12-31";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <filter type="and">
      <condition attribute="createdon" operator="between" value="2023-01-01,2023-12-31" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_complex_null_and_range_conditions() {
    let fql = ".opportunity | .primarycontactid != null and .estimatedvalue between 25000 and 100000 | order(.estimatedvalue desc)";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="opportunity">
    <filter type="and">
      <condition attribute="primarycontactid" operator="not-null" />
      <condition attribute="estimatedvalue" operator="between" value="25000,100000" />
    </filter>
    <order attribute="estimatedvalue" descending="true" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_multiple_null_checks() {
    let fql = ".account | .primarycontactid != null and .emailaddress1 == null";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false">
  <entity name="account">
    <filter type="and">
      <condition attribute="primarycontactid" operator="not-null" />
      <condition attribute="emailaddress1" operator="null" />
    </filter>
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}
