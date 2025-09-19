use super::test_fql_to_xml;

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