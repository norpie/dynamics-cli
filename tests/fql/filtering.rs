use super::test_fql_to_xml;

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
