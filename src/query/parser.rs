//! Query language parser using nom.

use super::ast::*;
use crate::core::{Result, UrpoError};
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while1},
    character::complete::{char, digit1, multispace0},
    combinator::{map, recognize, value as nom_value},
    multi::many0,
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};

/// Parse a query string into an AST
pub fn parse_query(input: &str) -> Result<Query> {
    let input = input.trim();

    if input.is_empty() {
        return Ok(Query {
            filter: QueryFilter::All,
        });
    }

    match query_filter(input) {
        Ok((remaining, filter)) => {
            if !remaining.trim().is_empty() {
                Err(UrpoError::Parse {
                    message: format!("Unexpected input after query: '{}'", remaining),
                })
            } else {
                Ok(Query { filter })
            }
        },
        Err(e) => Err(UrpoError::Parse {
            message: format!("Failed to parse query: {}", e),
        }),
    }
}

/// Parse a query filter (the main expression)
fn query_filter(input: &str) -> IResult<&str, QueryFilter> {
    logical_or(input)
}

/// Parse logical OR expressions
fn logical_or(input: &str) -> IResult<&str, QueryFilter> {
    let (input, first) = logical_and(input)?;

    let (input, rest) = many0(tuple((
        preceded(multispace0, tag("||")),
        preceded(multispace0, logical_and),
    )))(input)?;

    Ok((
        input,
        rest.into_iter()
            .fold(first, |acc, (_, right)| QueryFilter::Logical {
                op: LogicalOp::Or,
                left: Box::new(acc),
                right: Box::new(right),
            }),
    ))
}

/// Parse logical AND expressions
fn logical_and(input: &str) -> IResult<&str, QueryFilter> {
    let (input, first) = primary_filter(input)?;

    let (input, rest) = many0(tuple((
        preceded(multispace0, tag("&&")),
        preceded(multispace0, primary_filter),
    )))(input)?;

    Ok((
        input,
        rest.into_iter()
            .fold(first, |acc, (_, right)| QueryFilter::Logical {
                op: LogicalOp::And,
                left: Box::new(acc),
                right: Box::new(right),
            }),
    ))
}

/// Parse primary filter expressions
fn primary_filter(input: &str) -> IResult<&str, QueryFilter> {
    preceded(multispace0, alt((grouped_filter, comparison_filter)))(input)
}

/// Parse grouped (parenthesized) filters
fn grouped_filter(input: &str) -> IResult<&str, QueryFilter> {
    map(
        delimited(char('('), preceded(multispace0, query_filter), preceded(multispace0, char(')'))),
        |filter| QueryFilter::Group(Box::new(filter)),
    )(input)
}

/// Parse comparison filters
fn comparison_filter(input: &str) -> IResult<&str, QueryFilter> {
    map(
        tuple((field, preceded(multispace0, operator), preceded(multispace0, field_value))),
        |(field, op, value)| QueryFilter::Comparison { field, op, value },
    )(input)
}

/// Parse field names
fn field(input: &str) -> IResult<&str, Field> {
    alt((
        nom_value(Field::Service, tag_no_case("service")),
        nom_value(Field::Name, alt((tag_no_case("name"), tag_no_case("operation")))),
        nom_value(Field::Duration, tag_no_case("duration")),
        nom_value(Field::Status, tag_no_case("status")),
        nom_value(Field::TraceId, alt((tag_no_case("trace_id"), tag_no_case("traceid")))),
        nom_value(Field::SpanId, alt((tag_no_case("span_id"), tag_no_case("spanid")))),
        nom_value(
            Field::ParentSpanId,
            alt((tag_no_case("parent_span_id"), tag_no_case("parentspanid"))),
        ),
        nom_value(Field::SpanKind, tag_no_case("span.kind")),
        map(attribute_name, Field::Attribute),
    ))(input)
}

/// Parse attribute names (e.g., http.status_code, db.statement)
fn attribute_name(input: &str) -> IResult<&str, String> {
    map(
        recognize(pair(
            take_while1(|c: char| c.is_alphanumeric() || c == '_'),
            many0(pair(char('.'), take_while1(|c: char| c.is_alphanumeric() || c == '_'))),
        )),
        |s: &str| s.to_string(),
    )(input)
}

/// Parse operators
fn operator(input: &str) -> IResult<&str, Operator> {
    alt((
        nom_value(Operator::Regex, tag("=~")),
        nom_value(Operator::NotEq, tag("!=")),
        nom_value(Operator::Gte, tag(">=")),
        nom_value(Operator::Lte, tag("<=")),
        nom_value(Operator::Eq, tag("=")),
        nom_value(Operator::Gt, tag(">")),
        nom_value(Operator::Lt, tag("<")),
        nom_value(Operator::Contains, tag_no_case("contains")),
    ))(input)
}

/// Parse field values
fn field_value(input: &str) -> IResult<&str, Value> {
    alt((
        map(duration_value, Value::Duration),
        map(status_value, Value::Status),
        map(boolean_value, Value::Boolean),
        map(integer_value, Value::Integer),
        map(string_literal, Value::String),
    ))(input)
}

/// Parse string literals (quoted or unquoted for simple identifiers)
fn string_literal(input: &str) -> IResult<&str, String> {
    alt((
        // Quoted string
        map(delimited(char('"'), take_while1(|c| c != '"'), char('"')), |s: &str| {
            s.to_string()
        }),
        // Unquoted identifier
        map(
            take_while1(|c: char| {
                c.is_alphanumeric() || c == '_' || c == '-' || c == '/' || c == '.'
            }),
            |s: &str| s.to_string(),
        ),
    ))(input)
}

/// Parse duration values (e.g., 100ms, 1s, 5m)
fn duration_value(input: &str) -> IResult<&str, DurationValue> {
    map(pair(digit1, duration_unit), |(num_str, unit)| DurationValue {
        value: num_str.parse().unwrap_or(0),
        unit,
    })(input)
}

/// Parse duration units
fn duration_unit(input: &str) -> IResult<&str, DurationUnit> {
    alt((
        nom_value(DurationUnit::Nanoseconds, tag("ns")),
        nom_value(DurationUnit::Microseconds, alt((tag("us"), tag("μs")))),
        nom_value(DurationUnit::Milliseconds, tag("ms")),
        nom_value(DurationUnit::Seconds, tag("s")),
        nom_value(DurationUnit::Minutes, tag("m")),
    ))(input)
}

/// Parse status values
fn status_value(input: &str) -> IResult<&str, StatusValue> {
    alt((
        nom_value(StatusValue::Ok, tag_no_case("ok")),
        nom_value(StatusValue::Error, tag_no_case("error")),
        nom_value(StatusValue::Unknown, tag_no_case("unknown")),
    ))(input)
}

/// Parse boolean values
fn boolean_value(input: &str) -> IResult<&str, bool> {
    alt((nom_value(true, tag_no_case("true")), nom_value(false, tag_no_case("false"))))(input)
}

/// Parse integer values
fn integer_value(input: &str) -> IResult<&str, i64> {
    map(digit1, |s: &str| s.parse().unwrap_or(0))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_comparison() {
        let query = parse_query("service = api").unwrap();
        match query.filter {
            QueryFilter::Comparison { field, op, value } => {
                assert_eq!(field, Field::Service);
                assert_eq!(op, Operator::Eq);
                assert_eq!(value, Value::String("api".to_string()));
            },
            _ => panic!("Expected comparison filter"),
        }
    }

    #[test]
    fn test_parse_duration_comparison() {
        let query = parse_query("duration > 100ms").unwrap();
        match query.filter {
            QueryFilter::Comparison { field, op, value } => {
                assert_eq!(field, Field::Duration);
                assert_eq!(op, Operator::Gt);
                match value {
                    Value::Duration(d) => {
                        assert_eq!(d.value, 100);
                        assert_eq!(d.unit, DurationUnit::Milliseconds);
                    },
                    _ => panic!("Expected duration value"),
                }
            },
            _ => panic!("Expected comparison filter"),
        }
    }

    #[test]
    fn test_parse_logical_and() {
        let query = parse_query("service = api && duration > 100ms").unwrap();
        match query.filter {
            QueryFilter::Logical { op, .. } => {
                assert_eq!(op, LogicalOp::And);
            },
            _ => panic!("Expected logical filter"),
        }
    }

    #[test]
    fn test_parse_grouped_expression() {
        let query = parse_query("service = api && (status = error || duration > 1s)").unwrap();
        // Just check it parses without error
        assert!(matches!(query.filter, QueryFilter::Logical { .. }));
    }

    #[test]
    fn test_parse_attribute_query() {
        let query = parse_query("http.status_code = 500").unwrap();
        match query.filter {
            QueryFilter::Comparison { field, op, value } => {
                assert_eq!(field, Field::Attribute("http.status_code".to_string()));
                assert_eq!(op, Operator::Eq);
                assert_eq!(value, Value::Integer(500));
            },
            _ => panic!("Expected comparison filter"),
        }
    }
}
