use jev_protocol::Summary;

#[derive(Default)]
pub struct Query(Vec<(bool, String, String)>);

impl Query {
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut terms = vec![];
        let mut word = String::new();
        let mut quoted = false;
        for c in input.chars().chain(std::iter::once(' ')) {
            if c == '"' {
                quoted = !quoted;
            } else if c.is_whitespace() && !quoted {
                if !word.is_empty() {
                    terms.push(std::mem::take(&mut word));
                }
            } else {
                word.push(c);
            }
        }
        if quoted {
            return Err("Close the quote to finish your search.".into());
        }
        let mut query = Self::default();
        for term in terms {
            let (negate, term) = term
                .strip_prefix('-')
                .map_or((false, term.as_str()), |t| (true, t));
            let (field, value) = term.split_once(':').unwrap_or(("", term));
            if !matches!(
                field,
                "" | "status" | "model" | "method" | "path" | "id" | "request" | "duration"
            ) {
                return Err(format!(
                    "Unknown field ‘{field}’. Open Help for search syntax."
                ));
            }
            if value.is_empty() {
                return Err(format!("Add a value after {field}:."));
            }
            if field == "duration" && numeric(value, 0.0).is_none() {
                return Err("Use duration:>200, duration:>=500ms, or duration:>1s.".into());
            }
            if field == "status" && !status_valid(value) {
                return Err("Use status:200, status:4xx, or status:>=400.".into());
            }
            query.0.push((negate, field.into(), value.to_lowercase()));
        }
        Ok(query)
    }
    pub fn matches(&self, r: &Summary) -> bool {
        self.0.iter().all(|(negate, field, value)| {
            let matched = match field.as_str() {
                "status" => r.status.is_some_and(|n| {
                    if value.ends_with("xx") {
                        n / 100 == value[..1].parse::<u16>().unwrap_or(0)
                    } else {
                        numeric(value, f64::from(n)).unwrap_or(false)
                    }
                }),
                "duration" => r
                    .duration_ms
                    .is_some_and(|n| numeric(value, n).unwrap_or(false)),
                "model" => contains(r.model.as_deref().unwrap_or(""), value),
                "method" => r.method.eq_ignore_ascii_case(value),
                "path" => contains(&r.path, value),
                "id" => r.id.to_string() == *value,
                "request" => contains(r.request_id.as_deref().unwrap_or(""), value),
                _ => contains(
                    &format!(
                        "{} {} {} {} {} {} {:?}",
                        r.id,
                        r.method,
                        r.path,
                        r.model.as_deref().unwrap_or(""),
                        r.status.map(|v| v.to_string()).unwrap_or_default(),
                        r.request_id.as_deref().unwrap_or(""),
                        r.outcome
                    ),
                    value,
                ),
            };
            matched != *negate
        })
    }
}
fn contains(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(needle)
}
fn status_valid(value: &str) -> bool {
    let operand = [">=", "<=", ">", "<", "="]
        .iter()
        .find_map(|op| value.strip_prefix(op))
        .unwrap_or(value);
    (value.len() == 3 && value.ends_with("xx") && matches!(value.as_bytes()[0], b'1'..=b'5'))
        || (operand.len() == 3
            && operand.bytes().all(|b| b.is_ascii_digit())
            && operand
                .parse::<u16>()
                .is_ok_and(|v| (100..=599).contains(&v)))
}
fn numeric(input: &str, actual: f64) -> Option<bool> {
    let (op, value) = [">=", "<=", ">", "<", "="]
        .iter()
        .find_map(|op| input.strip_prefix(op).map(|v| (*op, v)))
        .unwrap_or(("=", input));
    let (value, multiplier) = if let Some(v) = value.strip_suffix("ms") {
        (v, 1.0)
    } else if let Some(v) = value.strip_suffix('s') {
        (v, 1000.0)
    } else {
        (value, 1.0)
    };
    let target = value.parse::<f64>().ok()? * multiplier;
    if !target.is_finite() || target < 0.0 {
        return None;
    }
    Some(match op {
        ">=" => actual >= target,
        "<=" => actual <= target,
        ">" => actual > target,
        "<" => actual < target,
        _ => actual == target,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combines_fields_quotes_negation_and_numeric_units() {
        let r = crate::demo::exchange(7).summary;
        assert!(
            Query::parse("status:4xx duration:>0.1s -model:other method:post \"systemone\"")
                .unwrap()
                .matches(&r)
        );
        assert!(!Query::parse("status:2xx").unwrap().matches(&r));
        assert!(!Query::parse("duration:>1s").unwrap().matches(&r));
        for bad in [
            "\"open",
            "status:",
            "duration:>NaN",
            "status:hello",
            "unknown:hi",
            "status:200ms",
            "status:0.2s",
            "status:200.5",
            "status:99",
            "status:600",
            "status:>=200.5",
            "status:>999",
        ] {
            assert!(Query::parse(bad).is_err(), "{bad}");
        }
        assert!(
            Query::parse("status:>=400 status:<500")
                .unwrap()
                .matches(&r)
        );
    }
}
