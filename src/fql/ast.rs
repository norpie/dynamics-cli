use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub entity: Entity,
    pub attributes: Vec<Attribute>,
    pub filters: Vec<Filter>,
    pub joins: Vec<Join>,
    pub order: Vec<OrderBy>,
    pub aggregations: Vec<Aggregation>,
    pub group_by: Vec<String>,
    pub having: Option<Filter>,
    pub limit: Option<u32>,
    pub page: Option<(u32, u32)>, // (page_number, page_size)
    pub distinct: bool,
    pub options: QueryOptions,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub alias: Option<String>,
    pub entity_alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    Condition {
        attribute: String,
        operator: FilterOperator,
        value: FilterValue,
        entity_alias: Option<String>,
    },
    And(Vec<Filter>),
    Or(Vec<Filter>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterOperator {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Like,
    NotLike,
    BeginsWith,
    EndsWith,
    In,
    NotIn,
    Between,
    Null,
    NotNull,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Date(String),
    List(Vec<FilterValue>),
    Range(Box<FilterValue>, Box<FilterValue>),
    RangeTraditional(Box<FilterValue>, Box<FilterValue>),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Join {
    pub entity: Entity,
    pub join_type: JoinType,
    pub on_condition: JoinCondition,
    pub filters: Vec<Filter>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JoinCondition {
    pub from_attribute: String,
    pub to_attribute: String,
    pub from_entity_alias: Option<String>,
    pub to_entity_alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Aggregation {
    pub function: AggregationFunction,
    pub attribute: Option<String>,
    pub alias: Option<String>,
    pub entity_alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggregationFunction {
    Count,
    Sum,
    Average,
    Min,
    Max,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    pub attribute: String,
    pub direction: OrderDirection,
    pub entity_alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueryOptions {
    pub no_lock: bool,
    pub return_total_record_count: bool,
    pub formatted: bool,
    pub custom_options: HashMap<String, String>,
}
