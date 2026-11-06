mod realm;
mod model;

use std::sync::OnceLock;
use topcoat::Result;
use topcoat::router::module_router;
use topcoat::runtime::procedure;

#[tokio::main]
async fn main() {
    topcoat::start(
        module_router!().route(assess_policy).build(),
    )
    .await
    .unwrap();
}

// ---------------------------------------------------------------------------
// Engine procedure
// ---------------------------------------------------------------------------

#[procedure("/api/assess")]
pub async fn assess_policy(criterion: String, context: String) -> Result<String> {
    Ok(evaluate_policy(&criterion, &context))
}

fn evaluate_policy(criterion: &str, context: &str) -> String {
    let policy = parse_policy(criterion);
    let candidate = parse_context(context);
    let result = model::engine::assess(&policy, &candidate);
    flat(&result)
}

// ---------------------------------------------------------------------------
// Policy / context DSL parsers
// ---------------------------------------------------------------------------

fn parse_policy(criterion: &str) -> model::engine::Policy {
    let mut rules = vec![];
    for raw in criterion.split(|c| c == ';' || c == '|') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let (rule, effect) = match raw.rsplit_once(':') {
            Some((r, e)) => (r, e),
            None => (raw, "permit"),
        };
        // rule shape: axis.attribute operator value
        let mut dots = rule.splitn(2, '.');
        let axis_s = dots.next().unwrap_or("subject");
        let tail = dots.next().unwrap_or("");
        let mut parts = tail.splitn(2, ' ');
        let attribute = parts.next().unwrap_or("").to_string();
        let remainder = parts.next().unwrap_or("");
        let (op_s, value_s) = match remainder.split_once(' ') {
            Some((o, v)) => (o, v),
            None => (remainder, ""),
        };
        rules.push(model::engine::Rule {
            axis: axis_of(axis_s),
            condition: model::engine::Condition {
                axis: axis_of(axis_s),
                attribute,
                operator: operator_of(op_s),
                value: parse_value(value_s),
            },
            effect: effect_of(effect),
        });
    }
    model::engine::Policy { rules }
}


fn axis_of(s: &str) -> model::engine::Axis {
    match s.trim().to_lowercase().as_str() {
        "subject" => model::engine::Axis::Subject,
        "object" => model::engine::Axis::Object,
        "environment" => model::engine::Axis::Environment,
        "action" => model::engine::Axis::Action,
        _ => model::engine::Axis::Subject,
    }
}

fn operator_of(s: &str) -> model::engine::Operator {
    match s.trim().to_lowercase().as_str() {
        "=" | "==" => model::engine::Operator::Eq,
        "!=" => model::engine::Operator::Ne,
        ">" => model::engine::Operator::Gt,
        ">=" => model::engine::Operator::Ge,
        "<" => model::engine::Operator::Lt,
        "in" => model::engine::Operator::In,
        "contains" => model::engine::Operator::Contains,
        "exists" => model::engine::Operator::Exists,
        _ => model::engine::Operator::Eq,
    }
}

fn parse_value(s: &str) -> model::engine::AttributeValue {
    let s = s.trim();
    if s.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        model::engine::AttributeValue::Integer(s.parse().unwrap_or(0))
    } else {
        model::engine::AttributeValue::String(s.to_string())
    }
}

impl std::fmt::Display for model::engine::AttributeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            model::engine::AttributeValue::String(s) => write!(f, "{}", s),
            model::engine::AttributeValue::Integer(i) => write!(f, "{}", i),
            model::engine::AttributeValue::Float(fl) => write!(f, "{}", fl),
            model::engine::AttributeValue::Bool(b) => write!(f, "{}", b),
        }
    }
}

fn effect_of(s: &str) -> model::engine::Effect {
    match s.trim().to_lowercase().as_str() {
        "deny" => model::engine::Effect::Deny,
        _ => model::engine::Effect::Permit,
    }
}

fn parse_context(s: &str) -> model::engine::Candidate {
    let mut subject = std::collections::HashMap::new();
    let mut object = std::collections::HashMap::new();
    let mut environment = std::collections::HashMap::new();
    let mut action = String::new();

    for raw in s.split(|c| c == ',' || c == ';') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        if let Some((axis, rest)) = raw.split_once('.') {
            if let Some((key, val)) = rest.split_once('=') {
                if key == "action" {
                    action = val.to_string();
                } else {
                    let v = parse_value(&val);
                    match axis {
                        "subject" => { subject.insert(key.to_string(), v); }
                        "object" => { object.insert(key.to_string(), v); }
                        "environment" => { environment.insert(key.to_string(), v); }
                        _ => {}
                    }
                }
            }
        }
    }

    model::engine::Candidate {
        subject,
        object,
        action,
        environment,
        action_value: OnceLock::new(),
    }
}

// ---------------------------------------------------------------------------
// Result serialization
// ---------------------------------------------------------------------------

fn flat(result: &model::engine::DecisionResult) -> String {
    let verdict = match result.verdict {
        model::engine::Decision::Permit => "permit",
        model::engine::Decision::Deny => "deny",
        model::engine::Decision::Indeterminate => "indeterminate",
    };
    let trace = result
        .trace
        .iter()
        .map(|t| {
            format!(
                "{}{}{}{}",
                t.rule_index, t.attribute, model::engine::operator_str(t.operator), t.value
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    format!("verdict={}|trace_count={}|trace={}", verdict, result.trace.len(), trace)
}

