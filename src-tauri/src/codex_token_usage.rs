use serde_json::{json, Map, Value};

#[derive(Debug, Default)]
pub(crate) struct CodexTokenCountState {
    previous_total: Option<CodexTokenUsageTotals>,
}

#[derive(Debug, Clone)]
pub(crate) struct CodexTokenUsageMetrics {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub cached_input_tokens: Option<i64>,
    pub reasoning_tokens: Option<i64>,
    pub raw_usage_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CodexTokenUsageTotals {
    input_tokens: i64,
    output_tokens: i64,
    cached_input_tokens: i64,
    reasoning_tokens: i64,
}

pub(crate) fn is_codex_token_count_event(root: &Value) -> bool {
    root.get("type").and_then(Value::as_str) == Some("event_msg")
        && root
            .get("payload")
            .and_then(|payload| payload.get("type"))
            .and_then(Value::as_str)
            == Some("token_count")
}

pub(crate) fn parse_codex_token_count_usage(
    root: &Value,
    state: &mut CodexTokenCountState,
) -> Option<CodexTokenUsageMetrics> {
    let info = codex_token_count_info(root)?;
    let total_usage = info
        .get("total_token_usage")
        .and_then(CodexTokenUsageTotals::from_usage_value);
    let last_usage = info
        .get("last_token_usage")
        .and_then(CodexTokenUsageTotals::from_usage_value);

    match (total_usage, last_usage, state.previous_total) {
        (Some(total), Some(last), Some(previous)) => {
            if total == previous {
                return None;
            }

            if total.delta_from(previous).is_none()
                && total.looks_like_stale_regression(previous, last)
            {
                return None;
            }

            state.previous_total = Some(total);
            Some(metrics_from_totals(
                last,
                "last_token_usage",
                info.get("last_token_usage"),
            ))
        }
        (Some(total), Some(last), None) => {
            state.previous_total = Some(total);
            Some(metrics_from_totals(
                last,
                "last_token_usage",
                info.get("last_token_usage"),
            ))
        }
        (Some(total), None, Some(previous)) => {
            if total == previous {
                return None;
            }

            if let Some(delta) = total.delta_from(previous) {
                state.previous_total = Some(total);
                return Some(metrics_from_totals(
                    delta,
                    "total_token_usage_delta",
                    info.get("total_token_usage"),
                ));
            }

            state.previous_total = Some(total);
            None
        }
        (Some(total), None, None) => {
            state.previous_total = Some(total);
            Some(metrics_from_totals(
                total,
                "total_token_usage",
                info.get("total_token_usage"),
            ))
        }
        (None, Some(last), Some(previous)) => {
            state.previous_total = Some(previous.saturating_add(last));
            Some(metrics_from_totals(
                last,
                "last_token_usage",
                info.get("last_token_usage"),
            ))
        }
        (None, Some(last), None) => Some(metrics_from_totals(
            last,
            "last_token_usage",
            info.get("last_token_usage"),
        )),
        (None, None, _) => None,
    }
    .filter(|metrics| !metrics.is_zero())
}

fn codex_token_count_info(root: &Value) -> Option<&Value> {
    if !is_codex_token_count_event(root) {
        return None;
    }

    root.get("payload")?.get("info")
}

impl CodexTokenUsageTotals {
    fn from_usage_value(usage: &Value) -> Option<Self> {
        let object = usage.as_object()?;
        let input_tokens = read_i64(object, &["input_tokens", "prompt_tokens"]).unwrap_or(0);
        let output_tokens = read_i64(object, &["output_tokens", "completion_tokens"]).unwrap_or(0);
        let cached_input_tokens = read_cached_input_tokens(usage).unwrap_or(0);
        let reasoning_tokens = nested_i64(
            usage,
            &[
                &["output_tokens_details", "reasoning_tokens"],
                &["completion_tokens_details", "reasoning_tokens"],
                &["reasoning_tokens"],
                &["reasoning_output_tokens"],
            ],
        )
        .unwrap_or(0);

        let totals = Self {
            input_tokens: input_tokens.max(0),
            output_tokens: output_tokens.max(0),
            cached_input_tokens: cached_input_tokens.max(0),
            reasoning_tokens: reasoning_tokens.max(0),
        };
        if totals.is_zero() {
            None
        } else {
            Some(totals)
        }
    }

    fn delta_from(self, previous: Self) -> Option<Self> {
        if self.input_tokens < previous.input_tokens
            || self.output_tokens < previous.output_tokens
            || self.cached_input_tokens < previous.cached_input_tokens
            || self.reasoning_tokens < previous.reasoning_tokens
        {
            return None;
        }

        Some(Self {
            input_tokens: self.input_tokens - previous.input_tokens,
            output_tokens: self.output_tokens - previous.output_tokens,
            cached_input_tokens: self.cached_input_tokens - previous.cached_input_tokens,
            reasoning_tokens: self.reasoning_tokens - previous.reasoning_tokens,
        })
    }

    fn saturating_add(self, other: Self) -> Self {
        Self {
            input_tokens: self.input_tokens.saturating_add(other.input_tokens),
            output_tokens: self.output_tokens.saturating_add(other.output_tokens),
            cached_input_tokens: self
                .cached_input_tokens
                .saturating_add(other.cached_input_tokens),
            reasoning_tokens: self.reasoning_tokens.saturating_add(other.reasoning_tokens),
        }
    }

    fn summed_fields(self) -> i64 {
        self.input_tokens
            .saturating_add(self.output_tokens)
            .saturating_add(self.cached_input_tokens)
            .saturating_add(self.reasoning_tokens)
    }

    fn is_zero(self) -> bool {
        self.input_tokens == 0
            && self.output_tokens == 0
            && self.cached_input_tokens == 0
            && self.reasoning_tokens == 0
    }

    fn looks_like_stale_regression(self, previous: Self, last: Self) -> bool {
        let previous_total = previous.summed_fields();
        let current_total = self.summed_fields();
        let last_total = last.summed_fields();
        if previous_total <= 0 || current_total <= 0 || last_total <= 0 {
            return false;
        }

        // Codex 有时会先写出略旧的累计快照，再恢复到更高水位。
        // 这种回退不能当成会话重置，否则会把 last_token_usage 重复计入。
        current_total.saturating_mul(100) >= previous_total.saturating_mul(98)
            || current_total.saturating_add(last_total.saturating_mul(2)) >= previous_total
    }
}

impl CodexTokenUsageMetrics {
    fn is_zero(&self) -> bool {
        self.input_tokens == 0
            && self.output_tokens == 0
            && self.cached_input_tokens.unwrap_or(0) == 0
            && self.reasoning_tokens.unwrap_or(0) == 0
    }
}

fn metrics_from_totals(
    totals: CodexTokenUsageTotals,
    source_kind: &str,
    source_usage: Option<&Value>,
) -> CodexTokenUsageMetrics {
    let cached_input_tokens = totals.cached_input_tokens.min(totals.input_tokens).max(0);
    let reasoning_tokens = totals.reasoning_tokens.max(0);
    CodexTokenUsageMetrics {
        input_tokens: totals.input_tokens.max(0),
        output_tokens: totals.output_tokens.max(0),
        total_tokens: totals
            .input_tokens
            .max(0)
            .saturating_add(totals.output_tokens.max(0)),
        cached_input_tokens: positive_metric(cached_input_tokens),
        reasoning_tokens: positive_metric(reasoning_tokens),
        raw_usage_json: serialize_usage_snapshot(source_kind, source_usage, totals),
    }
}

fn positive_metric(value: i64) -> Option<i64> {
    if value > 0 {
        Some(value)
    } else {
        None
    }
}

fn serialize_usage_snapshot(
    source_kind: &str,
    source_usage: Option<&Value>,
    totals: CodexTokenUsageTotals,
) -> String {
    let snapshot = json!({
        "source_kind": source_kind,
        "source_usage": source_usage,
        "normalized_usage": {
            "input_tokens": totals.input_tokens,
            "output_tokens": totals.output_tokens,
            "cached_input_tokens": totals.cached_input_tokens,
            "reasoning_tokens": totals.reasoning_tokens
        }
    });
    serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".to_string())
}

fn read_cached_input_tokens(usage: &Value) -> Option<i64> {
    let candidates = [
        nested_i64(usage, &[&["input_tokens_details", "cached_tokens"]]),
        nested_i64(usage, &[&["prompt_tokens_details", "cached_tokens"]]),
        nested_i64(usage, &[&["cached_input_tokens"]]),
        nested_i64(usage, &[&["cache_read_input_tokens"]]),
        nested_i64(usage, &[&["input_cached_tokens"]]),
    ];

    candidates.into_iter().flatten().max()
}

fn read_i64(object: &Map<String, Value>, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| object.get(*key))
        .and_then(value_to_i64)
}

fn nested_i64(value: &Value, paths: &[&[&str]]) -> Option<i64> {
    paths.iter().find_map(|path| {
        let mut current = value;
        for key in *path {
            current = current.get(key)?;
        }
        value_to_i64(current)
    })
}

fn value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        .or_else(|| value.as_f64().map(|number| number.round() as i64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn uses_last_usage_when_total_and_last_are_present() {
        let value = json!({
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "total_token_usage": {
                        "input_tokens": 100,
                        "cached_input_tokens": 20,
                        "output_tokens": 30,
                        "reasoning_output_tokens": 5
                    },
                    "last_token_usage": {
                        "input_tokens": 12,
                        "cached_input_tokens": 3,
                        "output_tokens": 4,
                        "reasoning_output_tokens": 1
                    }
                }
            }
        });
        let mut state = CodexTokenCountState::default();

        let metrics = parse_codex_token_count_usage(&value, &mut state).unwrap();

        assert_eq!(metrics.input_tokens, 12);
        assert_eq!(metrics.output_tokens, 4);
        assert_eq!(metrics.total_tokens, 16);
        assert_eq!(metrics.cached_input_tokens, Some(3));
        assert_eq!(metrics.reasoning_tokens, Some(1));
    }

    #[test]
    fn skips_repeated_total_snapshot() {
        let value = json!({
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "total_token_usage": {
                        "input_tokens": 100,
                        "cached_input_tokens": 20,
                        "output_tokens": 30
                    },
                    "last_token_usage": {
                        "input_tokens": 100,
                        "cached_input_tokens": 20,
                        "output_tokens": 30
                    }
                }
            }
        });
        let mut state = CodexTokenCountState::default();

        assert!(parse_codex_token_count_usage(&value, &mut state).is_some());
        assert!(parse_codex_token_count_usage(&value, &mut state).is_none());
    }

    #[test]
    fn uses_total_delta_when_last_usage_is_absent() {
        let first_value = json!({
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "total_token_usage": {
                        "input_tokens": 100,
                        "cached_input_tokens": 20,
                        "output_tokens": 30
                    }
                }
            }
        });
        let second_value = json!({
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "total_token_usage": {
                        "input_tokens": 115,
                        "cached_input_tokens": 25,
                        "output_tokens": 34
                    }
                }
            }
        });
        let mut state = CodexTokenCountState::default();

        let first_metrics = parse_codex_token_count_usage(&first_value, &mut state).unwrap();
        let second_metrics = parse_codex_token_count_usage(&second_value, &mut state).unwrap();

        assert_eq!(first_metrics.input_tokens, 100);
        assert_eq!(first_metrics.output_tokens, 30);
        assert_eq!(second_metrics.input_tokens, 15);
        assert_eq!(second_metrics.output_tokens, 4);
        assert_eq!(second_metrics.cached_input_tokens, Some(5));
    }
}
