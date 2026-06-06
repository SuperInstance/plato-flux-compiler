/// Condition AST types for the Plato alarm condition DSL.
use std::fmt;

/// Comparison operator used in simple alarm conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
}

impl fmt::Display for CmpOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CmpOp::Equal => write!(f, "=="),
            CmpOp::NotEqual => write!(f, "!="),
            CmpOp::LessThan => write!(f, "<"),
            CmpOp::LessEqual => write!(f, "<="),
            CmpOp::GreaterThan => write!(f, ">"),
            CmpOp::GreaterEqual => write!(f, ">="),
        }
    }
}

impl CmpOp {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "==" | "=" => Some(CmpOp::Equal),
            "!=" => Some(CmpOp::NotEqual),
            "<" => Some(CmpOp::LessThan),
            "<=" => Some(CmpOp::LessEqual),
            ">" => Some(CmpOp::GreaterThan),
            ">=" => Some(CmpOp::GreaterEqual),
            _ => None,
        }
    }
}

/// A simple comparison: `sensor_name op value`
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub sensor: String,
    pub op: CmpOp,
    pub value: i64,
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.sensor, self.op, self.value)
    }
}

/// Logical operators for combining conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}

impl fmt::Display for LogicalOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicalOp::And => write!(f, "AND"),
            LogicalOp::Or => write!(f, "OR"),
        }
    }
}

/// Range check: `sensor in [low, high]`
#[derive(Debug, Clone, PartialEq)]
pub struct RangeCheck {
    pub sensor: String,
    pub low: i64,
    pub high: i64,
}

impl fmt::Display for RangeCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} in [{}, {}]", self.sensor, self.low, self.high)
    }
}

/// Threshold check: `sensor rising_by delta in ticks_ticks`
/// Triggers when the sensor value has increased by at least `delta` over `ticks` ticks.
#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdCheck {
    pub sensor: String,
    pub delta: i64,
    pub ticks: u64,
}

impl fmt::Display for ThresholdCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} rising_by {} in {}_ticks",
            self.sensor, self.delta, self.ticks
        )
    }
}

/// A complete alarm condition — the root of the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Comparison(Comparison),
    Range(RangeCheck),
    Threshold(ThresholdCheck),
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Condition::Comparison(c) => write!(f, "{}", c),
            Condition::Range(r) => write!(f, "{}", r),
            Condition::Threshold(t) => write!(f, "{}", t),
            Condition::Not(inner) => write!(f, "NOT ({})", inner),
            Condition::And(l, r) => write!(f, "({} AND {})", l, r),
            Condition::Or(l, r) => write!(f, "({} OR {})", l, r),
        }
    }
}

impl Condition {
    /// Collect all unique sensor names referenced by this condition.
    pub fn sensors(&self) -> Vec<String> {
        let mut sensors = Vec::new();
        self.collect_sensors(&mut sensors);
        sensors.sort();
        sensors.dedup();
        sensors
    }

    fn collect_sensors(&self, sensors: &mut Vec<String>) {
        match self {
            Condition::Comparison(c) => {
                sensors.push(c.sensor.clone());
            }
            Condition::Range(r) => {
                sensors.push(r.sensor.clone());
            }
            Condition::Threshold(t) => {
                sensors.push(t.sensor.clone());
            }
            Condition::Not(inner) => inner.collect_sensors(sensors),
            Condition::And(l, r) | Condition::Or(l, r) => {
                l.collect_sensors(sensors);
                r.collect_sensors(sensors);
            }
        }
    }
}
