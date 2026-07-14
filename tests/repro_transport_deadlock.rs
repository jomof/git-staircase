#[cfg(test)]
mod tests {
    use git_staircase::git::GitRepo;
    use git_staircase::workspace::review_provider::{
        ProductionTransport, ProviderTransport, TransportRequest,
    };
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    #[test]
    fn test_transport_deadlock() {
        let tmp = TempDir::new().unwrap();
        let bin_dir = tmp.path().join("bin");
        fs::create_dir(&bin_dir).unwrap();
        let repo = GitRepo::new(tmp.path().to_path_buf());

        let tool_script = bin_dir.join("gh");
        fs::write(
            &tool_script,
            r#"#!/usr/bin/env python3
import sys
import os
import time

# Write a lot of data to stdout to fill the pipe.
data = b"X" * (128 * 1024)
try:
    os.write(sys.stdout.fileno(), data)
except BrokenPipeError:
    pass

# Now wait without reading from stdin.
time.sleep(2)
"#,
        )
        .unwrap();

        let mut perms = fs::metadata(&tool_script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&tool_script, perms).unwrap();

        // Add bin_dir to PATH
        let old_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", bin_dir.display(), old_path);
        unsafe {
            std::env::set_var("PATH", &new_path);
        }

        let transport = ProductionTransport::default();

        // Large body to send to stdin.
        let body = serde_json::json!({
            "large_data": "A".repeat(128 * 1024)
        });

        let request = TransportRequest::Api {
            tool: "gh".into(),
            method: "POST".into(),
            endpoint: "test".into(),
            arguments: vec![],
            body: Some(body),
        };

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = transport.execute(&repo, &request);
            tx.send(result).unwrap();
        });

        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(_) => println!("Test finished (may have failed but didn't hang)"),
            Err(_) => {
                // Restore PATH before panicking
                unsafe {
                    std::env::set_var("PATH", &old_path);
                }
                panic!("Test DEADLOCKED!");
            }
        }
        unsafe {
            std::env::set_var("PATH", old_path);
        }
    }
}
