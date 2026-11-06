use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Attribute values — the atomic values a candidate can carry.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
}

impl AttributeValue {
    pub fn as_string(&self) -> Option<&str> {
        match self { AttributeValue::String(s) => Some(s), _ => None }
    }
    pub fn as_integer(&self) -> Option<i64> {
        match self { AttributeValue::Integer(i) => Some(*i), _ => None }
    }
    pub fn as_float(&self) -> Option<f64> {
        match self { AttributeValue::Float(f) => Some(*f), _ => None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self { AttributeValue::Bool(b) => Some(*b), _ => None }
    }
    pub fn cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (AttributeValue::String(a), AttributeValue::String(b)) => Some(a.cmp(b)),
            (AttributeValue::Integer(a), AttributeValue::Integer(b)) => Some(a.cmp(b)),
            (AttributeValue::Float(a), AttributeValue::Float(b)) => a.partial_cmp(b),
            (AttributeValue::Bool(a), AttributeValue::Bool(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// The four attribute axes.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    Subject,
    Object,
    Action,
    Environment,
}

impl Axis {
    pub fn as_str(self) -> &'static str {
        match self {
            Axis::Subject => "subject",
            Axis::Object => "object",
            Axis::Action => "action",
            Axis::Environment => "environment",
        }
    }
}

// ---------------------------------------------------------------------------
// Operators used in a condition.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    In,
    Contains,
    Exists,
}

pub fn operator_str(op: Operator) -> &'static str {
    match op {
        Operator::Eq => "==",
        Operator::Ne => "!=",
        Operator::Gt => ">",
        Operator::Ge => ">=",
        Operator::Lt => "<",
        Operator::In => "in",
        Operator::Contains => "contains",
        Operator::Exists => "exists",
    }
}

// ---------------------------------------------------------------------------
// Conditions, rules, effects.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub axis: Axis,
    pub attribute: String,
    pub operator: Operator,
    pub value: AttributeValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Permit,
    Deny,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub axis: Axis,
    pub condition: Condition,
    pub effect: Effect,
}

// ---------------------------------------------------------------------------
// The access request (a candidate).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub subject: HashMap<String, AttributeValue>,
    pub object: HashMap<String, AttributeValue>,
    pub action: String,
    pub action_value: OnceLock<AttributeValue>,
    pub environment: HashMap<String, AttributeValue>,
}

impl Candidate {
    pub fn get(&self, axis: Axis, attribute: &str) -> Option<&AttributeValue> {
        match axis {
            Axis::Subject => self.subject.get(attribute),
            Axis::Object => self.object.get(attribute),
            Axis::Action => Some(&self.action_value.get_or_init(|| AttributeValue::String(self.action.clone()))),
            Axis::Environment => self.environment.get(attribute),
        }
    }
}

// ---------------------------------------------------------------------------
// Policy & decisions.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Policy {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Permit,
    Deny,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceEntry {
    pub rule_index: usize,
    pub axis: Axis,
    pub attribute: String,
    pub operator: Operator,
    pub value: AttributeValue,
    pub matched: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionResult {
    pub verdict: Decision,
    pub trace: Vec<TraceEntry>,
}

// Re-exported by `model/mod.rs`.

// ---------------------------------------------------------------------------
// Evaluation: given a policy and a candidate, produce a verdict + trace.
//
// Invocation contract: `assess(policy, candidate)`.
//
// Deny-overrides: a matched `Deny` rules the whole call out, even if other
// rules matched `Permit`. Otherwise a matched `Permit` grants access; otherwise
// the policy is inconclusive (`Indeterminate`). Rules are evaluated in order
// and every rule is recorded in the trace so callers can explain the verdict.
// ---------------------------------------------------------------------------

pub fn assess(policy: &Policy, candidate: &Candidate) -> DecisionResult {
    let mut trace = Vec::with_capacity(policy.rules.len());
    let mut matched_permit = false;
    let mut matched_deny = false;

    for (i, rule) in policy.rules.iter().enumerate() {
        let cond = &rule.condition;
        let matched = evaluate_condition(cond, candidate);
        let detail = format!("{:<12} {} {:?} ({})", cond.attribute, operator_str(cond.operator), cond.value, cond.axis.as_str());
        trace.push(TraceEntry {
            rule_index: i,
            axis: cond.axis,
            attribute: cond.attribute.clone(),
            operator: cond.operator,
            value: cond.value.clone(),
            matched,
            reason: if matched { format!("true:  {}", detail) } else { format!("false: {}", detail) },
        });

        if matched {
            matched_permit = matched_permit || rule.effect == Effect::Permit;
            matched_deny = matched_deny || rule.effect == Effect::Deny;
        }
    }

    let verdict = if matched_deny {
        Decision::Deny
    } else if matched_permit {
        Decision::Permit
    } else {
        Decision::Indeterminate
    };

    DecisionResult { verdict, trace }
}

fn evaluate_condition(cond: &Condition, candidate: &Candidate) -> bool {
    // `Candidate::get` resolves the `Action` axis to the request's action, so all
    // axes share one clean evaluation path.
    let lhs = candidate.get(cond.axis, &cond.attribute);
    evaluate_attr(lhs, cond)
}

fn evaluate_attr(lhs: Option<&AttributeValue>, cond: &Condition) -> bool {
    match lhs {
        None => false,
        Some(lhs) => match cond.operator {
            Operator::Exists => true,
            Operator::Eq => lhs == &cond.value,
            Operator::Ne => lhs != &cond.value,
            Operator::Gt => cmp_attr(lhs, &cond.value) == Some(Ordering::Greater),
            Operator::Ge => cmp_attr(lhs, &cond.value).map_or(false, |o| o == Ordering::Greater || o == Ordering::Equal),
            Operator::Lt => cmp_attr(lhs, &cond.value) == Some(Ordering::Less),
            Operator::In => lhs_string_and_list(cond, lhs),
            Operator::Contains => lhs_string_and_str(cond, lhs),
        },
    }
}

fn cmp_attr(a: &AttributeValue, b: &AttributeValue) -> Option<Ordering> {
    a.cmp(b)
}

fn lhs_string_and_list(cond: &Condition, lhs: &AttributeValue) -> bool {
    match (lhs.as_string(), cond.value.as_string()) {
        (Some(s), Some(list)) => list.split(',').any(|item| item.trim() == s),
        _ => false,
    }
}

fn lhs_string_and_str(cond: &Condition, lhs: &AttributeValue) -> bool {
    match (lhs.as_string(), cond.value.as_string()) {
        (Some(s), Some(sub)) => s.contains(sub),
        _ => false,
    }
}

#[cfg(test)]
mod assumptions {
    use super::*;

    fn policy(rules: Vec<Rule>) -> Policy {
        Policy { rules }
    }

    fn candidate(subject: HashMap<String, AttributeValue>, object: HashMap<String, AttributeValue>, action: String, environment: HashMap<String, AttributeValue>) -> Candidate {
        Candidate { subject, object, action, environment, action_value: OnceLock::new() }
    }

    // Assumption: zero matching conditions yields Indeterminate.
    #[test]
    fn assumes_zero_matching_conditions_are_indeterminate() {
        let p = policy(vec![Rule {
            axis: Axis::Subject,
            condition: Condition { axis: Axis::Subject, attribute: "role".into(), operator: Operator::Eq, value: AttributeValue::String("admin".into()) },
            effect: Effect::Permit,
        }]);
        let mut env = HashMap::new();
        env.insert("location".into(), AttributeValue::String("ward-a".into()));
        let c = candidate(HashMap::new(), HashMap::new(), "read".into(), env);
        let r = assess(&p, &c);
        assert_eq!(r.verdict, Decision::Indeterminate);
        assert_eq!(r.trace.len(), 1);
        assert!(!r.trace[0].matched);
    }

    // Assumption: deny wins over permit, even when a permit matched.
    #[test]
    fn assumes_deny_overrides_permit() {
        let p = policy(vec![
            Rule {
                axis: Axis::Subject,
                condition: Condition { axis: Axis::Subject, attribute: "role".into(), operator: Operator::Ne, value: AttributeValue::String("nobody".into()) },
                effect: Effect::Permit,
            },
            Rule {
                axis: Axis::Subject,
                condition: Condition { axis: Axis::Subject, attribute: "role".into(), operator: Operator::Eq, value: AttributeValue::String("hacker".into()) },
                effect: Effect::Deny,
            },
        ]);
        let mut env = HashMap::new();
        env.insert("location".into(), AttributeValue::String("ward-a".into()));
        let mut subject = HashMap::new();
        subject.insert("role".into(), AttributeValue::String("hacker".into()));
        let c = candidate(subject, HashMap::new(), "read".into(), env);
        let r = assess(&p, &c);
        assert_eq!(r.verdict, Decision::Deny);
    }

    // Assumption: environment conditions come from the candidate, not the host clock.
    #[test]
    fn assumes_environment_context_is_candidate_supplied_not_wallclock() {
        let p = policy(vec![Rule {
            axis: Axis::Environment,
            condition: Condition { axis: Axis::Environment, attribute: "time-of-day".into(), operator: Operator::Eq, value: AttributeValue::String("09:00".into()) },
            effect: Effect::Permit,
        }]);
        let mut env = HashMap::new();
        env.insert("time-of-day".into(), AttributeValue::String("09:00".into()));
        let c = candidate(HashMap::new(), HashMap::new(), "read".into(), env);
        let r = assess(&p, &c);
        assert_eq!(r.verdict, Decision::Permit);

        let mut env2 = HashMap::new();
        env2.insert("time-of-day".into(), AttributeValue::String("14:00".into()));
        let c2 = candidate(HashMap::new(), HashMap::new(), "read".into(), env2);
        let r2 = assess(&p, &c2);
        assert_eq!(r2.verdict, Decision::Indeterminate);
    }

    // Assumption: ordered comparisons work on comparable values.
    #[test]
    fn assumes_ordered_comparison_on_comparable_values() {
        let p = policy(vec![Rule {
            axis: Axis::Object,
            condition: Condition { axis: Axis::Object, attribute: "sensitivity".into(), operator: Operator::Gt, value: AttributeValue::Integer(5) },
            effect: Effect::Deny,
        }]);
        let mut object = HashMap::new();
        object.insert("sensitivity".into(), AttributeValue::Integer(7));
        let c = candidate(HashMap::new(), object, "read".into(), HashMap::new());
        let r = assess(&p, &c);
        assert_eq!(r.verdict, Decision::Deny);

        let mut object2 = HashMap::new();
        object2.insert("sensitivity".into(), AttributeValue::Integer(3));
        let c2 = candidate(HashMap::new(), object2, "read".into(), HashMap::new());
        let r2 = assess(&p, &c2);
        assert_eq!(r2.verdict, Decision::Indeterminate);
    }

    // Assumption: "in" matches a comma-separated list.
    #[test]
    fn assumes_in_matches_comma_separated_list() {
        let p = policy(vec![Rule {
            axis: Axis::Subject,
            condition: Condition { axis: Axis::Subject, attribute: "role".into(), operator: Operator::In, value: AttributeValue::String("admin,doctor,nurse".into()) },
            effect: Effect::Permit,
        }]);
        let mut subject = HashMap::new();
        subject.insert("role".into(), AttributeValue::String("doctor".into()));
        let c = candidate(subject, HashMap::new(), "read".into(), HashMap::new());
        let r = assess(&p, &c);
        assert_eq!(r.verdict, Decision::Permit);
    }

    // Assumption: "contains" matches a substring.
    #[test]
    fn assumes_contains_substring() {
        let p = policy(vec![Rule {
            axis: Axis::Subject,
            condition: Condition { axis: Axis::Subject, attribute: "name".into(), operator: Operator::Contains, value: AttributeValue::String("smith".into()) },
            effect: Effect::Permit,
        }]);
        let mut subject = HashMap::new();
        subject.insert("name".into(), AttributeValue::String("alice.smith".into()));
        let c = candidate(subject, HashMap::new(), "read".into(), HashMap::new());
        let r = assess(&p, &c);
        assert_eq!(r.verdict, Decision::Permit);
    }

    #[test]
    fn assumes_exists_fires_only_when_attribute_present() {
        let p = policy(vec![Rule {
            axis: Axis::Subject,
            condition: Condition { axis: Axis::Subject, attribute: "role".into(), operator: Operator::Exists, value: AttributeValue::String("anything".into()) },
            effect: Effect::Permit,
        }]);
        let c = candidate(HashMap::new(), HashMap::new(), "read".into(), HashMap::new());
        let r = assess(&p, &c);
        assert_eq!(r.verdict, Decision::Indeterminate);

        let mut subject = HashMap::new();
        subject.insert("role".into(), AttributeValue::String("doctor".into()));
        let c2 = candidate(subject, HashMap::new(), "read".into(), HashMap::new());
        let r2 = assess(&p, &c2);
        assert_eq!(r2.verdict, Decision::Permit);
    }

    // Golden master: a doctor may read a standard chart; a chart above clearance is denied.
    #[test]
    fn golden_master_atrium_lets_patron_in_and_blocks_overly_sensitive_chart() {
        let p = policy(vec![
            Rule {
                axis: Axis::Subject,
                condition: Condition { axis: Axis::Subject, attribute: "role".into(), operator: Operator::Eq, value: AttributeValue::String("doctor".into()) },
                effect: Effect::Permit,
            },
            Rule {
                axis: Axis::Object,
                condition: Condition { axis: Axis::Object, attribute: "sensitivity".into(), operator: Operator::Gt, value: AttributeValue::Integer(5) },
                effect: Effect::Deny,
            },
            Rule {
                axis: Axis::Object,
                condition: Condition { axis: Axis::Object, attribute: "type".into(), operator: Operator::Eq, value: AttributeValue::String("chart".into()) },
                effect: Effect::Permit,
            },
        ]);
        let mut subject = HashMap::new();
        subject.insert("role".into(), AttributeValue::String("doctor".into()));
        let mut object = HashMap::new();
        object.insert("type".into(), AttributeValue::String("chart".into()));
        object.insert("sensitivity".into(), AttributeValue::Integer(3));
        let c = candidate(subject, object, "read".into(), HashMap::new());
        let r = assess(&p, &c);
        assert_eq!(r.verdict, Decision::Permit);
        assert_eq!(r.trace.len(), 3);
    }

    #[test]
    fn golden_master_atrium_blocks_overly_sensitive_chart() {
        let p = policy(vec![
            Rule {
                axis: Axis::Subject,
                condition: Condition { axis: Axis::Subject, attribute: "role".into(), operator: Operator::Eq, value: AttributeValue::String("doctor".into()) },
                effect: Effect::Permit,
            },
            Rule {
                axis: Axis::Object,
                condition: Condition { axis: Axis::Object, attribute: "sensitivity".into(), operator: Operator::Gt, value: AttributeValue::Integer(5) },
                effect: Effect::Deny,
            },
            Rule {
                axis: Axis::Object,
                condition: Condition { axis: Axis::Object, attribute: "type".into(), operator: Operator::Eq, value: AttributeValue::String("chart".into()) },
                effect: Effect::Permit,
            },
        ]);
        let mut subject = HashMap::new();
        subject.insert("role".into(), AttributeValue::String("doctor".into()));
        let mut object = HashMap::new();
        object.insert("type".into(), AttributeValue::String("chart".into()));
        object.insert("sensitivity".into(), AttributeValue::Integer(7));
        let c = candidate(subject, object, "read".into(), HashMap::new());
        let r = assess(&p, &c);
        assert_eq!(r.verdict, Decision::Deny);
    }
}

