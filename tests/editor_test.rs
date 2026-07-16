use git_staircase::presentation::cli::edit_in_editor;
use std::env;
use std::fs;

#[test]
fn test_edit_in_editor_success() {
    // ARRANGE: Create a fake editor script that appends text to the file
    let temp_dir = env::temp_dir();
    let editor_script = temp_dir.join("fake_editor.sh");
    fs::write(&editor_script, "#!/bin/sh\necho ' edited' >> \"$1\"").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&editor_script, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let original_git_editor = env::var("GIT_EDITOR").ok();
    unsafe {
        env::set_var("GIT_EDITOR", &editor_script);
    }

    // ACT: Call edit_in_editor
    let initial = "initial";
    let result = edit_in_editor(initial, "TEST_EDIT", "txt");

    // CLEANUP: restore environment
    unsafe {
        if let Some(val) = original_git_editor {
            env::set_var("GIT_EDITOR", val);
        } else {
            env::remove_var("GIT_EDITOR");
        }
    }
    let _ = fs::remove_file(&editor_script);

    // ASSERT: verify content was edited
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "initial edited");
}

#[test]
fn test_edit_in_editor_failure() {
    // ARRANGE: Create a fake editor script that fails
    let temp_dir = env::temp_dir();
    let editor_script = temp_dir.join("failing_editor.sh");
    fs::write(&editor_script, "#!/bin/sh\nexit 1").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&editor_script, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let original_git_editor = env::var("GIT_EDITOR").ok();
    unsafe {
        env::set_var("GIT_EDITOR", &editor_script);
    }

    // ACT: Call edit_in_editor
    let result = edit_in_editor("initial", "TEST_FAIL", "txt");

    // CLEANUP: restore environment
    unsafe {
        if let Some(val) = original_git_editor {
            env::set_var("GIT_EDITOR", val);
        } else {
            env::remove_var("GIT_EDITOR");
        }
    }
    let _ = fs::remove_file(&editor_script);

    // ASSERT: verify it returned an error
    assert!(result.is_err());
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains("Editor exited with non-zero status")
    );
}
