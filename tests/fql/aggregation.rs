use super::test_fql_to_xml;

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
fn test_count_aggregation_contact_entity() {
    let fql = ".contact | count()";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" aggregate="true">
  <entity name="contact">
    <attribute name="contactid" aggregate="count" alias="count" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}

#[test]
fn test_count_aggregation_opportunity_entity() {
    let fql = ".opportunity | count()";
    let expected_xml = r#"<fetch version="1.0" output-format="xml-platform" mapping="logical" distinct="false" aggregate="true">
  <entity name="opportunity">
    <attribute name="opportunityid" aggregate="count" alias="count" />
  </entity>
</fetch>"#;

    test_fql_to_xml(fql, expected_xml).unwrap();
}