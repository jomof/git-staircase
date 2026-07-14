mod common;
use common::*;
use git_staircase::cli::{Presentation, ToPresentation, PresentationOutput, OutputFormat};
use serde::Serialize;

#[derive(Serialize)]
struct MockResult {
    name: String,
    value: i32,
}

impl ToPresentation for MockResult {
    fn to_presentation(&self) -> Presentation {
        Presentation::Section {
            title: "Mock Result".to_string(),
            children: vec![
                Presentation::Field {
                    label: "Name".to_string(),
                    value: self.name.clone(),
                },
                Presentation::Field {
                    label: "Value".to_string(),
                    value: self.value.to_string(),
                },
            ],
        }
    }
}

#[test]
fn test_presentation_output_trait() {
    let result = MockResult { name: "test".to_string(), value: 42 };
    let ctx = TestContext::new();
    let repo = &ctx.repo;
    
    // Just verify it doesn't panic
    result.present(OutputFormat::Human, repo).unwrap();
    result.present(OutputFormat::Json, repo).unwrap();
    result.present(OutputFormat::Porcelain, repo).unwrap();
}

#[derive(Serialize)]
struct TableResult {
    rows: Vec<Vec<String>>,
}

impl ToPresentation for TableResult {
    fn to_presentation(&self) -> Presentation {
        Presentation::Table {
            name: Some("Test Table".to_string()),
            rows: self.rows.clone(),
        }
    }
}

#[test]
fn test_presentation_output_table() {
    let result = TableResult {
        rows: vec![
            vec!["col1".to_string(), "col2".to_string()],
            vec!["val1".to_string(), "val2".to_string()],
        ],
    };
    let ctx = TestContext::new();
    let repo = &ctx.repo;
    
    result.present(OutputFormat::Human, repo).unwrap();
    result.present(OutputFormat::Porcelain, repo).unwrap();
}
