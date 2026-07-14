use crate::error::Result;
use crate::git::GitRepo;
use crate::model::{StaircaseMetadata, StaircaseUserMetadata, Step};
use std::collections::BTreeMap;

pub const POLICY_EXTENSION: &str = "git-staircase.policies";
pub const DISCOVERY_EXTENSION: &str = "git-staircase.discovery-overrides";
pub const ANCHOR_EXTENSION: &str = "git-staircase.internal.integration-anchor";
pub const STRUCTURAL_STATE_EXTENSION: &str = "git-staircase.internal.structural-state";
pub const GERRIT_EXTENSION: &str = "git-staircase.gerrit";
pub const GITHUB_EXTENSION: &str = "git-staircase.github";

pub fn serialize_descriptor(repo: &GitRepo, metadata: &StaircaseMetadata) -> Result<String> {
    serialize_structure(repo, metadata, &StaircaseUserMetadata::default())
}

pub fn serialize_structure(
    repo: &GitRepo,
    metadata: &StaircaseMetadata,
    user_metadata: &StaircaseUserMetadata,
) -> Result<String> {
    let object_format = repo.get_object_format()?;
    let target_oid = user_metadata
        .extensions
        .get(ANCHOR_EXTENSION)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .map(Ok)
        .unwrap_or_else(|| repo.resolve_commit(&metadata.target))?;
    let typed_oid = |hex: String| {
        serde_json::json!({
            "algorithm": object_format,
            "hex": hex,
        })
    };
    let steps = metadata
        .steps
        .iter()
        .map(|step| {
            let materializing_refs = step
                .branch
                .as_ref()
                .map(|branch| {
                    vec![if branch.starts_with("refs/") {
                        branch.clone()
                    } else {
                        format!("refs/heads/{}", branch)
                    }]
                })
                .unwrap_or_default();
            serde_json::json!({
                "id": step.id,
                "name": step.name,
                "cut_oid": typed_oid(step.cut.clone()),
                "materializing_refs": materializing_refs,
                "owned_refs": materializing_refs,
            })
        })
        .collect::<Vec<_>>();
    let policies = user_metadata
        .extensions
        .get(POLICY_EXTENSION)
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let discovery_overrides = user_metadata
        .extensions
        .get(DISCOVERY_EXTENSION)
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let structural_state = user_metadata
        .extensions
        .get(STRUCTURAL_STATE_EXTENSION)
        .cloned()
        .unwrap_or_else(|| serde_json::json!({"kind": "clean"}));
    let symbolic_targets = metadata
        .target
        .starts_with("refs/")
        .then(|| vec![metadata.target.clone()])
        .unwrap_or_default();
    let mut extensions = serde_json::Map::new();
    extensions.insert(
        "git-staircase.core".into(),
        serde_json::json!({
            "landing_policy": metadata.landing_policy,
            "verification_policy": metadata.verification_policy,
        }),
    );
    for key in [GERRIT_EXTENSION, GITHUB_EXTENSION] {
        if let Some(provider_state) = user_metadata.extensions.get(key) {
            extensions.insert(key.into(), provider_state.clone());
        }
    }
    let value = serde_json::json!({
        "schema": "git-staircase/structure",
        "version": 1,
        "kind": "linear",
        "object_format": object_format,
        "lineage_id": metadata.id,
        "integration_context": {
            "kind": "single-anchor",
            "anchors": [typed_oid(target_oid)],
            "symbolic_targets": symbolic_targets,
        },
        "steps": steps,
        "structural_state": structural_state,
        "layout": {
            "kind": metadata.primary_branch_layout.as_deref().unwrap_or("none"),
            "base": metadata.branch_layout_base,
        },
        "policies": policies,
        "discovery_overrides": discovery_overrides,
        "extensions": extensions,
        "parent_structure_revision_oid": null,
    });
    Ok(format!("{}\n", super::canonical_json(&value)?))
}

pub fn parse_descriptor(content: &str) -> Result<StaircaseMetadata> {
    parse_structure(content).map(|(metadata, _)| metadata)
}

pub fn parse_structure(
    content: &str,
) -> Result<(StaircaseMetadata, BTreeMap<String, serde_json::Value>)> {
    let value: serde_json::Value = serde_json::from_str(content.trim_end())?;
    use crate::error::StaircaseError;
    if value.get("schema").and_then(|value| value.as_str()) != Some("git-staircase/structure")
        || value.get("version").and_then(|value| value.as_u64()) != Some(1)
    {
        return Err(StaircaseError::Other(
            "unsupported structure schema; expected git-staircase/structure version 1".into(),
        ));
    }
    if value.get("kind").and_then(|value| value.as_str()) != Some("linear") {
        return Err(StaircaseError::UnsupportedTopology {
            operation: "read-structure".into(),
            reason: "only linear generation-1 structures are supported".into(),
        });
    }
    let object_format = required_json_str(&value, "object_format")?;
    let id = required_json_str(&value, "lineage_id")?.to_string();
    let context = value
        .get("integration_context")
        .ok_or_else(|| StaircaseError::Other("structure missing integration_context".into()))?;
    let target = context
        .get("symbolic_targets")
        .and_then(|value| value.as_array())
        .and_then(|values| values.first())
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            context
                .get("anchors")
                .and_then(|value| value.as_array())
                .and_then(|values| values.first())
                .and_then(|value| value.get("hex"))
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .ok_or_else(|| StaircaseError::Other("structure has no integration anchor".into()))?;
    let integration_anchor = context
        .get("anchors")
        .and_then(|value| value.as_array())
        .and_then(|values| values.first())
        .and_then(|value| value.get("hex"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| StaircaseError::Other("structure has no integration anchor".into()))?
        .to_string();
    let step_values = value
        .get("steps")
        .and_then(|value| value.as_array())
        .ok_or_else(|| StaircaseError::Other("structure steps must be an array".into()))?;
    let mut steps = Vec::new();
    for step in step_values {
        let id = required_json_str(step, "id")?.to_string();
        let name = required_json_str(step, "name")?.to_string();
        let cut_oid = step
            .get("cut_oid")
            .ok_or_else(|| StaircaseError::Other("step missing cut_oid".into()))?;
        if required_json_str(cut_oid, "algorithm")? != object_format {
            return Err(StaircaseError::Other(
                "step cut object format does not match structure".into(),
            ));
        }
        let cut = required_json_str(cut_oid, "hex")?.to_string();
        let branch = step
            .get("materializing_refs")
            .and_then(|value| value.as_array())
            .and_then(|values| values.first())
            .and_then(|value| value.as_str())
            .map(|reference| {
                reference
                    .strip_prefix("refs/heads/")
                    .unwrap_or(reference)
                    .to_string()
            });
        if id.is_empty() || name.is_empty() || cut.is_empty() {
            return Err(StaircaseError::Other(
                "structure contains an empty or incomplete step".into(),
            ));
        }
        steps.push(Step {
            id,
            name,
            cut,
            branch,
        });
    }
    if steps.is_empty() {
        return Err(StaircaseError::Other(
            "structure must contain at least one step".into(),
        ));
    }
    let layout = value.get("layout");
    let layout_kind = layout
        .and_then(|layout| layout.get("kind"))
        .and_then(|value| value.as_str())
        .unwrap_or("none");
    let primary_branch_layout = (layout_kind != "none").then(|| layout_kind.to_string());
    let branch_layout_base = layout
        .and_then(|layout| layout.get("base"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let core_extensions = value
        .get("extensions")
        .and_then(|value| value.get("git-staircase.core"));
    let landing_policy = core_extensions
        .and_then(|value| value.get("landing_policy"))
        .cloned()
        .filter(|value| !value.is_null())
        .map(serde_json::from_value)
        .transpose()?;
    let verification_policy = core_extensions
        .and_then(|value| value.get("verification_policy"))
        .cloned()
        .filter(|value| !value.is_null())
        .map(serde_json::from_value)
        .transpose()?;
    let mut structural_extensions = BTreeMap::new();
    structural_extensions.insert(
        POLICY_EXTENSION.into(),
        value
            .get("policies")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})),
    );
    structural_extensions.insert(
        DISCOVERY_EXTENSION.into(),
        value
            .get("discovery_overrides")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    );
    structural_extensions.insert(
        ANCHOR_EXTENSION.into(),
        serde_json::Value::String(integration_anchor),
    );
    structural_extensions.insert(
        STRUCTURAL_STATE_EXTENSION.into(),
        value
            .get("structural_state")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({"kind": "clean"})),
    );
    if let Some(extensions) = value.get("extensions").and_then(|value| value.as_object()) {
        for key in [GERRIT_EXTENSION, GITHUB_EXTENSION] {
            if let Some(provider_state) = extensions.get(key) {
                structural_extensions.insert(key.into(), provider_state.clone());
            }
        }
    }
    Ok((
        StaircaseMetadata {
            landing_policy,
            id,
            name: String::new(),
            target,
            steps,
            verification_policy,
            primary_branch_layout,
            branch_layout_base,
            user_metadata: None,
            lifecycle: None,
        },
        structural_extensions,
    ))
}

fn required_json_str<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str> {
    use crate::error::StaircaseError;
    value
        .get(key)
        .and_then(|value| value.as_str())
        .ok_or_else(|| StaircaseError::Other(format!("structure field '{}' must be a string", key)))
}
