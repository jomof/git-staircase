use std::process::Command;
fn main() {
    let mut cmd = Command::new("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    for (key, val) in cmd.get_envs() {
        println!("{:?} {:?}", key, val);
    }
}
