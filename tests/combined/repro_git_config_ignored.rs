
use crate::common::*;

#[test]
fn test_git_config_global_ignored() {
    let ctx = TestContext::new();

    let config_file = ctx.path().join("fake_global_config");
    std::fs::write(&config_file, "[user]\n    name = GlobalUser\n").unwrap();

    // Set GIT_CONFIG_GLOBAL in the process environment.
    // GitRepo::git_cmd() is expected to OVERWRITE this with /dev/null, which is the bug.
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", &config_file);
    }

    // We expect this to return "GlobalUser", but it will fail or return something else if the bug exists.
    let output = ctx.repo.run(&["config", "user.name"]);

    match output {
        Ok(val) => {
            assert_eq!(
                val, "GlobalUser",
                "GitRepo failed to pick up the global config!"
            );
        }
        Err(e) => {
            panic!(
                "GitRepo failed to read config (likely ignored global config): {}",
                e
            );
        }
    }
}
