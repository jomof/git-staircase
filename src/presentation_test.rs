#[cfg(test)]
mod tests {
    use crate::cli::formatting::{ToHuman, ToPorcelain};
    use crate::model::Step;

    #[test]
    fn test_step_presentation() {
        let step = Step {
            id: "step-123".to_string(),
            name: "My Step".to_string(),
            cut: "abcdef1234567890".to_string(),
            branch: None,
        };

        let human = step.to_human();
        assert_eq!(human, "My Step (abcdef1)");

        let porcelain = step.to_porcelain();
        assert_eq!(
            porcelain,
            "step\t1\t\"step-123\"\t\"My Step\"\t\"abcdef1234567890\"\n"
        );
    }
}
