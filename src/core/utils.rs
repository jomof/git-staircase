use crate::model::Step;

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

pub fn check_sequential_layout(steps: &[Step]) -> Option<String> {
    if steps.is_empty() {
        return None;
    }
    let tip_step = &steps[steps.len() - 1];
    let Some(ref base) = tip_step.branch else {
        return None;
    };

    // Check if other steps match
    for (i, step) in steps.iter().enumerate().take(steps.len() - 1) {
        let expected = format!("{}-{}", base, i + 1);
        if step.branch.as_deref() != Some(&expected) {
            return None;
        }
    }
    Some(base.clone())
}

#[cfg(test)]
mod tests {
    use super::{common_prefix, check_sequential_layout};
    use crate::model::Step;

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

    #[test]
    fn test_check_sequential_layout() {
        let step = |name: &str, branch: Option<&str>| Step {
            id: String::new(),
            name: name.to_string(),
            cut: String::new(),
            branch: branch.map(|b| b.to_string()),
        };

        assert_eq!(
            check_sequential_layout(&[step("s1", Some("feat"))]).as_deref(),
            Some("feat")
        );

        assert_eq!(
            check_sequential_layout(&[
                step("s1", Some("feat-1")),
                step("s2", Some("feat")),
            ]).as_deref(),
            Some("feat")
        );

        assert_eq!(
            check_sequential_layout(&[
                step("s1", Some("feat-1")),
                step("s2", Some("feat-2")),
                step("s3", Some("feat")),
            ]).as_deref(),
            Some("feat")
        );

        assert_eq!(
            check_sequential_layout(&[
                step("s1", Some("feat-1")),
                step("s2", Some("feat-3")),
                step("s3", Some("feat")),
            ]),
            None
        );

        assert_eq!(
            check_sequential_layout(&[
                step("s1", Some("feat-1")),
                step("s2", Some("feat-2")),
            ]),
            None
        );

        assert_eq!(
            check_sequential_layout(&[
                step("s1", Some("feat-1")),
                step("s2", None),
            ]),
            None
        );
    }
}
