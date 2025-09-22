//! Abstract Syntax Tree for the query language.

use std::fmt;

/// Root query structure
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub filter: QueryFilter,
}

/// Query filter expressions
#[derive(Debug, Clone, PartialEq)]
pub enum QueryFilter {
    /// Simple comparison: field op value
    Comparison {
        field: Field,
        op: Operator,
        value: Value,
    },
    /// Logical combination of filters
    Logical {
        op: LogicalOp,
        left: Box<QueryFilter>,
        right: Box<QueryFilter>,
    },
    /// Parenthesized expression
    Group(Box<QueryFilter>),
    /// Match all (empty query)
    All,
}

/// Field types that can be queried
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    /// Service name
    Service,
    /// Operation/span name
    Name,
    /// Span duration
    Duration,
    /// Span status (ok/error/unknown)
    Status,
    /// Trace ID
    TraceId,
    /// Span ID
    SpanId,
    /// Parent span ID
    ParentSpanId,
    /// Span kind (server/client/producer/consumer/internal)
    SpanKind,
    /// Custom attribute
    Attribute(String),
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    /// Equals
    Eq,
    /// Not equals
    NotEq,
    /// Greater than
    Gt,
    /// Greater than or equal
    Gte,
    /// Less than
    Lt,
    /// Less than or equal
    Lte,
    /// Regex match
    Regex,
    /// Contains substring
    Contains,
}

/// Query values
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// String literal
    String(String),
    /// Integer (for counts, status codes)
    Integer(i64),
    /// Duration in various units
    Duration(DurationValue),
    /// Boolean
    Boolean(bool),
    /// Span status
    Status(StatusValue),
}

/// Duration value with unit
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DurationValue {
    pub value: u64,
    pub unit: DurationUnit,
}

impl DurationValue {
    /// Convert to nanoseconds
    pub fn to_nanos(&self) -> u64 {
        match self.unit {
            DurationUnit::Nanoseconds => self.value,
            DurationUnit::Microseconds => self.value * 1_000,
            DurationUnit::Milliseconds => self.value * 1_000_000,
            DurationUnit::Seconds => self.value * 1_000_000_000,
            DurationUnit::Minutes => self.value * 60_000_000_000,
        }
    }

    /// Convert to microseconds (for storage compatibility)
    pub fn to_micros(&self) -> u64 {
        self.to_nanos() / 1_000
    }
}

/// Duration units
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DurationUnit {
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Seconds,
    Minutes,
}

/// Span status values
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusValue {
    Ok,
    Error,
    Unknown,
}

/// Logical operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogicalOp {
    And,
    Or,
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Field::Service => write!(f, "service"),
            Field::Name => write!(f, "name"),
            Field::Duration => write!(f, "duration"),
            Field::Status => write!(f, "status"),
            Field::TraceId => write!(f, "trace_id"),
            Field::SpanId => write!(f, "span_id"),
            Field::ParentSpanId => write!(f, "parent_span_id"),
            Field::SpanKind => write!(f, "span.kind"),
            Field::Attribute(name) => write!(f, "{}", name),
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operator::Eq => write!(f, "="),
            Operator::NotEq => write!(f, "!="),
            Operator::Gt => write!(f, ">"),
            Operator::Gte => write!(f, ">="),
            Operator::Lt => write!(f, "<"),
            Operator::Lte => write!(f, "<="),
            Operator::Regex => write!(f, "=~"),
            Operator::Contains => write!(f, "contains"),
        }
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.filter)
    }
}

impl fmt::Display for QueryFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryFilter::Comparison { field, op, value } => {
                write!(f, "{} {} {}", field, op, value)
            },
            QueryFilter::Logical { op, left, right } => {
                write!(f, "{} {} {}", left, op, right)
            },
            QueryFilter::Group(inner) => write!(f, "({})", inner),
            QueryFilter::All => write!(f, "*"),
        }
    }
}

impl fmt::Display for LogicalOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicalOp::And => write!(f, "&&"),
            LogicalOp::Or => write!(f, "||"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Duration(d) => write!(f, "{}{}", d.value, d.unit),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Status(s) => write!(f, "{}", s),
        }
    }
}

impl fmt::Display for DurationUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DurationUnit::Nanoseconds => write!(f, "ns"),
            DurationUnit::Microseconds => write!(f, "us"),
            DurationUnit::Milliseconds => write!(f, "ms"),
            DurationUnit::Seconds => write!(f, "s"),
            DurationUnit::Minutes => write!(f, "m"),
        }
    }
}

impl fmt::Display for StatusValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatusValue::Ok => write!(f, "ok"),
            StatusValue::Error => write!(f, "error"),
            StatusValue::Unknown => write!(f, "unknown"),
        }
    }
}
