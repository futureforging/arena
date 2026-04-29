//! Parse operator-delivered private sets (JSON and prose shapes).

use serde_json::Value;

/// Extract `Vec<u32>` from operator message `content`.
///
/// Accepts:
/// - JSON: top-level array, or object with `private_set` / `set` / `userSet` / `items` / `data`.
/// - Prose with a brace list, e.g. `Your private set is: {277, 322, ...}.`
pub fn parse_private_set(content: &str) -> Option<Vec<u32>> {
    let trimmed = content.trim();

    let try_array = |arr: &[Value]| -> Option<Vec<u32>> {
        arr.iter()
            .map(|x| {
                x.as_u64()
                    .and_then(|n| u32::try_from(n).ok())
            })
            .collect()
    };

    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        let nums = v
            .as_array()
            .and_then(|arr| try_array(arr))
            .or_else(|| {
                ["private_set", "set", "userSet", "items", "data"]
                    .into_iter()
                    .find_map(|key| {
                        v.get(key)
                            .and_then(|val| match val {
                                Value::Array(arr) => try_array(arr),
                                _ => None,
                            })
                    })
            });
        if let Some(nums) = nums {
            return Some(nums);
        }
    }

    parse_private_set_from_brace_list(trimmed)
}

/// Comma-separated unsigned integers inside the first `... { ... } ...` block.
fn parse_private_set_from_brace_list(content: &str) -> Option<Vec<u32>> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end <= start {
        return None;
    }
    let inner = &content[start + 1..end];
    let mut out = Vec::new();
    for part in inner.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        let n: u32 = t
            .parse()
            .ok()?;
        out.push(n);
    }
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::parse_private_set;

    #[test]
    fn operator_prose_brace_list_from_runtime_log_shape() {
        let s = "Your private set is: {277, 322, 425, 558, 625, 637, 664, 709, 727, 793, 803, 811, 827}.";
        assert_eq!(
            parse_private_set(s),
            Some(vec![277, 322, 425, 558, 625, 637, 664, 709, 727, 793, 803, 811, 827])
        );
    }

    #[test]
    fn json_top_level_array_still_parsed() {
        assert_eq!(parse_private_set("[1, 2, 3]"), Some(vec![1, 2, 3]));
    }

    #[test]
    fn json_object_with_private_set_key() {
        let s = r#"{"private_set": [10, 20]}"#;
        assert_eq!(parse_private_set(s), Some(vec![10, 20]));
    }
}
