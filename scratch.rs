use git2::Repository;

fn test_git2() {
    let repo = Repository::open(".").unwrap();
    let oid1 = repo.revparse_single("main").unwrap().id();
    let oid2 = repo.revparse_single("refs/heads/main^{commit}").unwrap().id();
    println!("oid1: {}", oid1);
    println!("oid2: {}", oid2);
}
fn main() { test_git2(); }
