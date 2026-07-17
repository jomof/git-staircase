use git2::Repository;

fn test_git2() {
    let repo = Repository::open(".").unwrap();
    let obj = repo.revparse_single("HEAD:Cargo.toml").unwrap();
    println!("type: {:?}", obj.kind());
    let blob = obj.as_blob().unwrap();
    println!("size: {}", blob.size());
}
fn main() { test_git2(); }
