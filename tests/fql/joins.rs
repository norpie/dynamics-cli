use super::test_fql_to_xml;

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
