// Copyright (c) Facebook, Inc. and its affiliates. All Rights Reserved
//! Sweep expansion for multirun configurations

use crate::core::{Override, OverrideValue, ParsedElement};

/// Expand sweep overrides into individual override sets.
///
/// Given a list of overrides like:
/// - db=mysql,postgresql
/// - server=dev,prod
///
/// Returns a list of override sets (cartesian product):
/// - [db=mysql, server=dev]
/// - [db=mysql, server=prod]
/// - [db=postgresql, server=dev]
/// - [db=postgresql, server=prod]
pub fn expand_sweeps(overrides: &[Override]) -> Vec<Vec<String>> {
    // Collect sweep dimensions
    let mut dimensions: Vec<Vec<String>> = Vec::new();

    for ovr in overrides {
        let key = &ovr.key.key_or_group;

        match &ovr.value {
            Some(OverrideValue::ChoiceSweep(cs)) => {
                // Expand choice sweep
                let choices: Vec<String> = cs
                    .list
                    .iter()
                    .map(|elem| format!("{}={}", key, element_to_string(elem)))
                    .collect();
                dimensions.push(choices);
            }
            Some(OverrideValue::RangeSweep(rs)) => {
                // Expand range sweep
                let start = rs.start.unwrap_or(0.0);
                let stop = rs.stop.unwrap_or(10.0);
                let step = rs.step;

                let mut choices = Vec::new();
                let mut current = start;
                while current < stop {
                    if step == step.floor() && current == current.floor() {
                        // Integer range
                        choices.push(format!("{}={}", key, current as i64));
                    } else {
                        choices.push(format!("{}={}", key, current));
                    }
                    current += step;
                }
                dimensions.push(choices);
            }
            Some(OverrideValue::Element(elem)) => {
                // Non-sweep override - single value
                dimensions.push(vec![format!("{}={}", key, element_to_string(elem))]);
            }
            None => {
                // Delete override
                dimensions.push(vec![format!("~{}", key)]);
            }
            _ => {
                // Other sweep types not yet supported
                dimensions.push(vec![format!("{}=<unsupported>", key)]);
            }
        }
    }

    // Cartesian product of all dimensions
    cartesian_product(&dimensions)
}

/// Compute cartesian product of all dimensions
fn cartesian_product(dimensions: &[Vec<String>]) -> Vec<Vec<String>> {
    if dimensions.is_empty() {
        return vec![vec![]];
    }

    if dimensions.len() == 1 {
        return dimensions[0].iter().map(|s| vec![s.clone()]).collect();
    }

    let first = &dimensions[0];
    let rest = &dimensions[1..];

    let rest_product = cartesian_product(rest);

    let mut result = Vec::new();
    for item in first {
        for rest_combo in &rest_product {
            let mut combo = vec![item.clone()];
            combo.extend(rest_combo.iter().cloned());
            result.push(combo);
        }
    }

    result
}

/// Convert a ParsedElement to its string representation
fn element_to_string(elem: &ParsedElement) -> String {
    match elem {
        ParsedElement::Null => "null".to_string(),
        ParsedElement::Bool(b) => b.to_string(),
        ParsedElement::Int(i) => i.to_string(),
        ParsedElement::Float(f) => f.to_string(),
        ParsedElement::String(s) => s.clone(),
        ParsedElement::QuotedString(qs) => {
            let quote = match qs.quote {
                crate::core::Quote::Single => '\'',
                crate::core::Quote::Double => '"',
            };
            format!("{}{}{}", quote, qs.text, quote)
        }
        ParsedElement::List(items) => {
            let inner: Vec<String> = items.iter().map(element_to_string).collect();
            format!("[{}]", inner.join(","))
        }
        ParsedElement::Dict(pairs) => {
            let inner: Vec<String> = pairs
                .iter()
                .map(|(k, v)| format!("{}:{}", k, element_to_string(v)))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
    }
}

/// Expand sweep strings without full parsing.
///
/// For simple sweeps like "db=mysql,postgresql", expands directly from strings.
/// This is faster for simple cases.
pub fn expand_simple_sweeps(overrides: &[&str]) -> Vec<Vec<String>> {
    let mut dimensions: Vec<Vec<String>> = Vec::new();

    for ovr in overrides {
        if let Some(eq_pos) = ovr.find('=') {
            let key = &ovr[..eq_pos];
            let value = &ovr[eq_pos + 1..];

            // Check for range sweep FIRST (since it contains commas)
            if value.starts_with("range(") && value.ends_with(')') {
                // Range sweep: a=range(1,10)
                let inner = &value[6..value.len() - 1];
                let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

                let start: i64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
                let stop: i64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
                let step: i64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);

                let choices: Vec<String> = (start..stop)
                    .step_by(step.max(1) as usize)
                    .map(|v| format!("{}={}", key, v))
                    .collect();
                dimensions.push(choices);
            } else if value.contains(',') && !value.starts_with('[') && !value.starts_with('{') {
                // Simple choice sweep: a=1,2,3
                let choices: Vec<String> = value
                    .split(',')
                    .map(|v| format!("{}={}", key, v.trim()))
                    .collect();
                dimensions.push(choices);
            } else {
                // Non-sweep value
                dimensions.push(vec![ovr.to_string()]);
            }
        } else {
            // Not a key=value override (maybe a delete or flag)
            dimensions.push(vec![ovr.to_string()]);
        }
    }

    cartesian_product(&dimensions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_sweep_expansion() {
        let overrides = vec!["db=mysql,postgres", "server=dev,prod"];
        let result = expand_simple_sweeps(&overrides);

        assert_eq!(result.len(), 4);
        assert!(result.contains(&vec!["db=mysql".to_string(), "server=dev".to_string()]));
        assert!(result.contains(&vec!["db=mysql".to_string(), "server=prod".to_string()]));
        assert!(result.contains(&vec!["db=postgres".to_string(), "server=dev".to_string()]));
        assert!(result.contains(&vec!["db=postgres".to_string(), "server=prod".to_string()]));
    }

    #[test]
    fn test_range_sweep_expansion() {
        let overrides = vec!["x=range(1,4)"];
        let result = expand_simple_sweeps(&overrides);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], vec!["x=1".to_string()]);
        assert_eq!(result[1], vec!["x=2".to_string()]);
        assert_eq!(result[2], vec!["x=3".to_string()]);
    }

    #[test]
    fn test_mixed_sweep_and_static() {
        let overrides = vec!["db=mysql,postgres", "port=3306"];
        let result = expand_simple_sweeps(&overrides);

        assert_eq!(result.len(), 2);
        assert!(result.contains(&vec!["db=mysql".to_string(), "port=3306".to_string()]));
        assert!(result.contains(&vec!["db=postgres".to_string(), "port=3306".to_string()]));
    }

    #[test]
    fn test_single_override() {
        let overrides = vec!["db=mysql"];
        let result = expand_simple_sweeps(&overrides);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], vec!["db=mysql".to_string()]);
    }

    #[test]
    fn test_empty_overrides() {
        let overrides: Vec<&str> = vec![];
        let result = expand_simple_sweeps(&overrides);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Vec::<String>::new());
    }

    #[test]
    fn test_large_sweep() {
        let overrides = vec!["a=1,2,3", "b=4,5,6", "c=7,8,9"];
        let result = expand_simple_sweeps(&overrides);

        // 3 * 3 * 3 = 27 combinations
        assert_eq!(result.len(), 27);
    }

    #[test]
    fn test_cartesian_product() {
        let dims = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["1".to_string(), "2".to_string()],
        ];
        let result = cartesian_product(&dims);

        assert_eq!(result.len(), 4);
        assert!(result.contains(&vec!["a".to_string(), "1".to_string()]));
        assert!(result.contains(&vec!["a".to_string(), "2".to_string()]));
        assert!(result.contains(&vec!["b".to_string(), "1".to_string()]));
        assert!(result.contains(&vec!["b".to_string(), "2".to_string()]));
    }
}
