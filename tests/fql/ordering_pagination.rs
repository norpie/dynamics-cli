use super::test_fql_to_xml;

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
