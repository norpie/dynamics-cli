use super::test_fql_to_xml;

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