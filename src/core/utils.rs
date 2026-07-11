pub fn common_prefix(names: &[&str]) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let first = names[0];
    let mut len = first.len();
    for name in &names[1..] {
        let shared = first
            .chars()
            .zip(name.chars())
            .take_while(|(a, b)| a == b)
            .count();
        len = len.min(shared);
        if len == 0 {
            return None;
        }
    }
    let prefix: String = first.chars().take(len).collect();
    let trimmed = prefix.trim_end_matches(['/', '-', '_', '.', ' ']);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::common_prefix;

    #[test]
    fn test_common_prefix() {
        assert_eq!(
            common_prefix(&["feat/a", "feat/b"]).as_deref(),
            Some("feat")
        );
        assert_eq!(
            common_prefix(&["step-1", "step-2", "step-3"]).as_deref(),
            Some("step")
        );
        assert_eq!(
            common_prefix(&["feature-x", "feature-y"]).as_deref(),
            Some("feature")
        );
        assert_eq!(common_prefix(&["alice", "bob"]), None);
        assert_eq!(common_prefix(&["/a", "/b"]), None);
        assert_eq!(common_prefix(&[]), None);
    }
}
