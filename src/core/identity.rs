use crate::error::Result;
use crate::git::GitRepo;
use crate::model::IdentityKind;

use super::ResolvedStaircase;
pub fn compute_identity(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    kind: IdentityKind,
) -> Result<String> {
    let mut metadata = staircase.metadata().clone();
    if kind == IdentityKind::Lineage && !staircase.is_managed() {
        metadata = super::resolved::adopt(repo, &metadata)?;
    }
    match kind {
        IdentityKind::Lineage => Ok(metadata.id.clone()),
        IdentityKind::Nominal => Ok(metadata.name.clone()),
        IdentityKind::Revision => {
            let format = repo.get_object_format()?;
            let target_oid = repo.resolve_commit(&metadata.target)?;
            let mut data = format!("format:{}\ntarget:{}\n", format, target_oid);
            for (i, step) in metadata.steps.iter().enumerate() {
                data.push_str(&format!("step{}:{}\n", i, step.cut));
            }
            repo.hash_data(&data)
        }
        IdentityKind::Body => {
            let target_oid = repo.resolve_commit(&metadata.target)?;
            let top_oid = metadata
                .steps
                .last()
                .map(|s| s.cut.as_str())
                .unwrap_or(&target_oid);
            let data = format!("target:{}\ntop:{}\n", target_oid, top_oid);
            repo.hash_data(&data)
        }
        IdentityKind::Decomposition => {
            let target_oid = repo.resolve_commit(&metadata.target)?;
            let mut patches = Vec::new();
            let mut last_cut = target_oid;
            for step in &metadata.steps {
                let patch_id = repo.get_patch_id(&last_cut, &step.cut)?;
                patches.push(patch_id);
                last_cut = step.cut.clone();
            }
            repo.hash_data(&patches.join("\n---\n"))
        }
        IdentityKind::Outcome => {
            let target_oid = repo.resolve_commit(&metadata.target)?;
            let target_tree = repo.get_tree_id(&target_oid)?;
            let top_oid = metadata
                .steps
                .last()
                .map(|s| s.cut.as_str())
                .unwrap_or(&target_oid);
            let top_tree = repo.get_tree_id(top_oid)?;
            let data = format!("base-tree:{}\ntop-tree:{}\n", target_tree, top_tree);
            repo.hash_data(&data)
        }
        IdentityKind::PatchSeries => {
            let target_oid = repo.resolve_commit(&metadata.target)?;
            let mut patch_ids = Vec::new();
            let mut last_cut = target_oid;
            for step in &metadata.steps {
                let patch_id = repo.get_patch_id(&last_cut, &step.cut)?;
                patch_ids.push(patch_id);
                last_cut = step.cut.clone();
            }
            repo.hash_data(&patch_ids.join("\n"))
        }
        IdentityKind::Review => Ok("".to_string()),
    }
}

