// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod common;
use common::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use git_staircase::memoization::MemoKey;

#[test]
fn test_memoize_helper_execution_count() {
    let ctx = TestContext::new();
    
    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    // ARRANGE: Define a key
    let key = MemoKey::ObjectFormat;
    
    // ACT: Call memoize twice
    let res1: String = ctx.repo.memoize(Some(key.clone()), || {
        call_count_clone.fetch_add(1, Ordering::SeqCst);
        Ok("sha1".to_string())
    }).unwrap();
    
    let res2: String = ctx.repo.memoize(Some(key.clone()), || {
        call_count_clone.fetch_add(1, Ordering::SeqCst);
        Ok("sha1".to_string())
    }).unwrap();
    
    // ASSERT: Result is same, but call count is 1
    assert_eq!(res1, "sha1");
    assert_eq!(res2, "sha1");
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_memoize_rev_head_not_cached() {
    let ctx = TestContext::new();
    
    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    // ACT: Call memoize_rev with "HEAD" twice
    let _ = ctx.repo.memoize_rev("HEAD", |r| MemoKey::ResolveCommit { rev: r.to_string() }, || {
        call_count_clone.fetch_add(1, Ordering::SeqCst);
        Ok("oid1".to_string())
    }).unwrap();
    
    let _ = ctx.repo.memoize_rev("HEAD", |r| MemoKey::ResolveCommit { rev: r.to_string() }, || {
        call_count_clone.fetch_add(1, Ordering::SeqCst);
        Ok("oid1".to_string())
    }).unwrap();
    
    // ASSERT: HEAD should NOT be cached, so call count is 2
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

#[test]
fn test_memoize_cleared_on_update() {
    let ctx = TestContext::new();
    let c1 = ctx.commit("f1.txt", "c1", "m1");
    
    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    let key = MemoKey::ResolveCommit { rev: "feat".to_string() };
    
    // ACT: Memoize
    let _ = ctx.repo.memoize(Some(key.clone()), || {
        call_count_clone.fetch_add(1, Ordering::SeqCst);
        Ok(c1.clone())
    }).unwrap();
    
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
    
    // ACT: Clear via update_branch
    ctx.repo.update_branch("feat", &c1).unwrap();
    
    // ACT: Memoize again
    let _ = ctx.repo.memoize(Some(key.clone()), || {
        call_count_clone.fetch_add(1, Ordering::SeqCst);
        Ok(c1.clone())
    }).unwrap();
    
    // ASSERT: Should have been called again
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}
