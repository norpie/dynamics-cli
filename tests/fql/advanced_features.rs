use super::test_fql_to_xml;

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