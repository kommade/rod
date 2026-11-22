//! Tokio-compatibility tests for the async validation pipeline.
//!
//! The other async tests drive `validate_async` with the hand-rolled
//! `support::block_on` executor, whose no-op waker only copes with
//! instantly-ready futures. These tests run against a **real** Tokio
//! runtime: `async_check`/`async_check_all` closures that actually
//! `await` (timer), futures handed to `tokio::spawn` (proving the
//! `Send`-future / `Self: Sync` contract end to end), and concurrent
//! validations via `tokio::join!`.

use crate::prelude::*;
use std::sync::Arc;
use std::time::Duration;

/// Database-lookup-shaped check: awaits a real timer before deciding.
/// (The closure snapshots the input into an owned value before the
/// `async move` block: borrowing the `&String` parameter directly inside
/// the block trips rustc's closure/HRTB inference — "lifetime may not
/// live long enough".)
#[derive(RodValidate)]
struct DbSignup {
    #[rod(
        String { length: 3..=12 },
        async_check = |u: &String| {
            let u = u.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                !u.eq_ignore_ascii_case("root")
            }
        }
    )]
    username: String,
}

/// Multi-issue check whose awaited future suspends between issues —
/// impossible to validate with a no-op-waker executor.
#[derive(RodValidate)]
struct TokioMultiIssue {
    #[rod(
        String { length: 5..=12 },
        async_check_all = |u: &String| {
            let u = u.clone();
            async move {
                let mut issues = Vec::new();
                tokio::time::sleep(Duration::from_millis(5)).await;
                if !u.starts_with("user_") {
                    issues.push("must start with user_");
                }
                // Yield to the reactor mid-collection: a no-op waker would
                // spin forever here; Tokio schedules the timer and comes back.
                tokio::time::sleep(Duration::from_millis(5)).await;
                if u.chars().any(|c| c.is_ascii_uppercase()) {
                    issues.push("must be lowercase");
                }
                issues
            }
        }
    )]
    username: String,
}

#[tokio::test]
async fn async_check_awaits_real_io() {
    let ok = DbSignup {
        username: "user_kim".to_string(),
    };
    // Sync pass (async checks don't run there)...
    assert!(
        ok.validate_all().is_ok(),
        "{}",
        ok.validate_all().unwrap_err()
    );
    // ...and the awaited lookup accepts it under the real reactor.
    assert!(
        ok.validate_async().await.is_ok(),
        "{}",
        ok.validate_async().await.unwrap_err()
    );

    let bad = DbSignup {
        username: "ROOT".to_string(), // length 4 ok, lookup rejects "root"
    };
    assert!(bad.validate_all().is_ok());
    let errors = bad.validate_async().await.unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::CheckFailed(path) if *path == "username"
    ));
}

#[tokio::test]
async fn validate_async_future_spawns_on_tokio() {
    // `tokio::spawn` demands the future be `Send + 'static`: the future
    // borrows `&self`, so the value travels via `Arc` (the type is `Sync`,
    // per the `where Self: Sync` bound on `validate_async`).
    let user = Arc::new(DbSignup {
        username: "tok_user".to_string(),
    });
    let handle = tokio::spawn(async move {
        let user = user;
        user.validate_async().await
    });
    assert!(handle.await.unwrap().is_ok());

    // Same for the failure path.
    let user = Arc::new(DbSignup {
        username: "root".to_string(),
    });
    let handle = tokio::spawn(async move { user.validate_async().await });
    let errors = handle.await.unwrap().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
}

#[tokio::test]
async fn async_check_all_collects_across_awaits() {
    let user = TokioMultiIssue {
        username: "BadHandle".to_string(), // length ok; both async issues fire
    };
    assert!(
        user.validate_all().is_ok(),
        "{}",
        user.validate_all().unwrap_err()
    );

    let errors = user.validate_async().await.unwrap_err();
    assert_eq!(errors.len(), 2, "{}", errors);
    for expected in ["must start with user_", "must be lowercase"] {
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, RodValidateError::UserDefined(msg) if *msg == expected)),
            "missing `{}` in {}",
            expected,
            errors
        );
    }

    // Empty issue vec after awaiting still means Ok.
    let ok = TokioMultiIssue {
        username: "user_kim".to_string(),
    };
    assert!(ok.validate_async().await.is_ok());
}

#[tokio::test]
async fn concurrent_validations_join() {
    // Two independent values validated concurrently on the runtime.
    let a = Arc::new(DbSignup {
        username: "user_a".to_string(),
    });
    let b = Arc::new(DbSignup {
        username: "user_b".to_string(),
    });
    let (ra, rb) = tokio::join!(a.validate_async(), b.validate_async());
    assert!(ra.is_ok(), "{}", ra.unwrap_err());
    assert!(rb.is_ok(), "{}", rb.unwrap_err());
}

#[tokio::test]
async fn spawned_task_reports_group_aware_async_errors() {
    // Groups + async under spawn: the grouped async field participates via
    // validate_async (which validates everything), regardless of the group.
    #[derive(RodValidate)]
    struct Account {
        #[rod(
            group = "account",
            String { length: 4..=20 },
            async_check = |t: &String| {
                let t = t.clone();
                async move {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    t.starts_with("tok_")
                }
            }
        )]
        token: String,
    }

    let account = Arc::new(Account {
        token: "bad_prefix".to_string(), // length ok, async check rejects
    });
    let handle = tokio::spawn(async move { account.validate_async().await });
    let errors = handle.await.unwrap().unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(matches!(
        errors.iter().next().unwrap(),
        RodValidateError::CheckFailed(path) if *path == "token"
    ));
}
