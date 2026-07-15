use crate::error::Result;
use crate::git::GitRepo;
use crate::model::IdentityKind;
use crate::workspace::bootstrap::{BootstrapOptions, bootstrap};
use crate::workspace::gerrit_provider::GerritProvider;
use crate::workspace::github_provider::GitHubProvider;
use crate::workspace::review_provider::ReviewProvider;

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
            let target_oid = repo.resolve_commit(&metadata.symbolic_integration_target)?;
            let mut data = format!("format:{}\ntarget:{}\n", format, target_oid);
            for (i, step) in metadata.steps.iter().enumerate() {
                data.push_str(&format!("step{}:{}\n", i, step.cut));
            }
            let hash = repo.hash_data(&data)?;
            Ok(format!("{}:{}", format, hash))
        }
        IdentityKind::Body => {
            let target_oid = repo.resolve_commit(&metadata.symbolic_integration_target)?;
            let top_oid = metadata
                .steps
                .last()
                .map(|s| s.cut.as_str())
                .unwrap_or(&target_oid);
            let data = format!("target:{}\ntop:{}\n", target_oid, top_oid);
            repo.hash_data(&data)
        }
        IdentityKind::Decomposition => {
            let target_oid = repo.resolve_commit(&metadata.symbolic_integration_target)?;
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
            let target_oid = repo.resolve_commit(&metadata.symbolic_integration_target)?;
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
            let target_oid = repo.resolve_commit(&metadata.symbolic_integration_target)?;
            let mut patch_ids = Vec::new();
            let mut last_cut = target_oid;
            for step in &metadata.steps {
                let patch_id = repo.get_patch_id(&last_cut, &step.cut)?;
                patch_ids.push(patch_id);
                last_cut = step.cut.clone();
            }
            repo.hash_data(&patch_ids.join("\n"))
        }
        IdentityKind::Review => {
            let boot_res = bootstrap(
                repo,
                &BootstrapOptions {
                    no_bootstrap: true,
                    ..Default::default()
                },
            )?;

            let providers: Vec<Box<dyn ReviewProvider>> =
                vec![Box::new(GerritProvider), Box::new(GitHubProvider)];

            let mut identifiers = Vec::new();
            let oids: Vec<String> = staircase
                .metadata()
                .steps
                .iter()
                .map(|s| s.cut.clone())
                .collect();

            for provider in providers {
                if let Some(instance) = provider.probe(repo, Some(&boot_res.record))? {
                    identifiers = instance.get_stable_identifiers(repo, &oids, None)?;
                    break;
                }
            }

            if identifiers.is_empty() {
                for oid in &oids {
                    let msg = repo.run(&["log", "-1", "--format=%B", oid])?;
                    match crate::workspace::gerrit_provider::parse_change_ids(&msg) {
                        crate::workspace::gerrit_provider::ChangeIdParseResult::Single(id) => {
                            identifiers.push(Some(id))
                        }
                        crate::workspace::gerrit_provider::ChangeIdParseResult::Multiple(ids) => {
                            identifiers.push(ids.first().cloned())
                        }
                        _ => identifiers.push(None),
                    }
                }
            }

            let data = identifiers
                .iter()
                .map(|opt| opt.as_deref().unwrap_or("<none>"))
                .collect::<Vec<_>>()
                .join("\n");
            repo.hash_data(&data)
        }
    }
}
