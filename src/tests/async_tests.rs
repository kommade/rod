//! Tests for `async_check`/`async_check_all` fields and the
//! `AsyncValidator` trait.

use crate::prelude::*;
use super::support::block_on;

/// A regular closure returning a `Send` future.
#[derive(RodValidate)]
struct SyncClosureUser {
    #[rod(
        String {
            length: 5..=12,
        },
        async_check = |u: &String| std::future::ready(u.starts_with("user_"))
    )]
    username: String,
}

/// An async closure (`async |x| ...`).
#[derive(RodValidate)]
struct AsyncClosureUser {
    #[rod(
        String {
            length: 6..=20,
        },
        async_check = async |u: &String| u.contains("verified")
    )]
    username: String,
}

/// Multiple async checks mixed with a failing sync rule.
#[derive(RodValidate)]
struct MultiCheckOrder {
    #[rod(
        i32 {
            size: 1..=10,
        },
        async_check = |v: &i32| std::future::ready(v % 2 == 0)
    )]
    amount: i32,
    #[rod(
        String {
            length: 3,
        },
        async_check = |s: &String| std::future::ready(s.ends_with("!"))
    )]
    greeting: String,
}

/// The `message:` attribute applies to async checks too.
#[derive(RodValidate)]
struct WithMessage {
    #[rod(
        String {
            length: 2..=10,
        },
        message: "username is reserved",
        async_check = |u: &String| std::future::ready(!u.eq_ignore_ascii_case("root"))
    )]
    username: String,
}

/// Enums: async checks run against the active variant's fields only.
#[derive(RodValidate)]
enum Payment {
    Card {
        #[rod(
            group = "details",
            String {
                length: 4,
            },
            async_check = |pan: &String| std::future::ready(pan.chars().all(|c| c.is_ascii_digit()))
        )]
        number: String,
    },
    Cash,
}

#[test]
fn sync_closure_async_check_ok() {
    let user = SyncClosureUser {
        username: "user_1".to_string(),
    };
    assert!(user.validate().is_ok(), "{}", user.validate().unwrap_err());
    let result = block_on(user.validate_async());
    assert!(result.is_ok(), "{}", result.unwrap_err());
}

#[test]
fn sync_closure_async_check_fails() {
    let user = SyncClosureUser {
        username: "test_1".to_string(),
    };
    // The sync validation passes...
    assert!(user.validate().is_ok());
    assert!(user.validate_all().is_ok());
    // ...but the async check rejects it.
    let errors = block_on(user.validate_async()).unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(errors
        .iter()
        .any(|e| matches!(e, RodValidateError::CheckFailed(path) if *path == "username")), "{}", errors);
}

#[test]
fn async_closure_check() {
    let good = AsyncClosureUser {
        username: "user_verified".to_string(),
    };
    assert!(block_on(good.validate_async()).is_ok());

    let bad = AsyncClosureUser {
        username: "user_denied".to_string(),
    };
    assert!(block_on(bad.validate_async()).is_err());
}

#[test]
fn async_checks_combine_with_sync_errors() {
    // Sync rules: amount in 1..=10 ok, greeting length 3 ok.
    // Async checks: amount % 2 == 0 fails, greeting ends_with "!" fails.
    let order = MultiCheckOrder {
        amount: 3,
        greeting: "hi".to_string(), // also fails the sync length rule
    };
    // validate_all reports only the sync failure.
    let sync_errors = order.validate_all().unwrap_err();
    assert_eq!(sync_errors.len(), 1, "{}", sync_errors);

    // validate_async reports sync + async failures combined.
    let errors = block_on(order.validate_async()).unwrap_err();
    assert_eq!(errors.len(), 3, "{}", errors);
    assert!(errors.iter().any(|e| matches!(e, RodValidateError::CheckFailed(path) if *path == "amount")));
    assert!(errors.iter().any(|e| matches!(e, RodValidateError::CheckFailed(path) if *path == "greeting")));
    assert!(errors.iter().any(|e| matches!(e, RodValidateError::String(_))));
}

#[test]
fn message_applies_to_async_check() {
    let user = WithMessage {
        username: "ROOT".to_string(),
    };
    assert!(user.validate().is_ok());
    let errors = block_on(user.validate_async()).unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(errors
        .iter()
        .any(|e| matches!(e, RodValidateError::UserDefined(msg) if msg == "username is reserved")), "{}", errors);
}

#[test]
fn enum_async_checks_only_run_for_active_variant() {
    let card_bad = Payment::Card {
        number: "12ab".to_string(), // length 4 ok, but async check fails
    };
    let errors = block_on(card_bad.validate_async()).unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(errors
        .iter()
        .any(|e| matches!(e, RodValidateError::CheckFailed(path) if *path == "number")), "{}", errors);

    // Unit variant: no async checks to run.
    let cash = Payment::Cash;
    assert!(block_on(cash.validate_async()).is_ok());
}

#[test]
fn sync_methods_do_not_run_async_checks() {
    let user = SyncClosureUser {
        username: "test_1".to_string(), // fails async check only
    };
    assert!(user.validate().is_ok());
    assert!(user.validate_all().is_ok());
    // ...only `validate_async` executes them.
    assert!(block_on(user.validate_async()).is_err());
}

#[test]
fn validate_async_future_is_send() {
    fn assert_send<T: Send>(_: &T) {}

    let user = SyncClosureUser {
        username: "user_1".to_string(),
    };
    let fut = user.validate_async();
    assert_send(&fut);
    assert!(block_on(fut).is_ok());
}

#[test]
fn run_async_checks_default_has_no_errors() {
    // Types without `async_check` get an empty override-less impl.
    #[derive(RodValidate)]
    struct Plain {
        #[rod(String { length: 2 })]
        name: String,
    }

    let plain = Plain {
        name: "ok".to_string(),
    };
    assert!(block_on(plain.run_async_checks()).is_empty());
    assert!(block_on(plain.validate_async()).is_ok());
}

#[test]
fn double_reference_async_check() {
    static TOKEN: &str = "tok123";

    #[derive(RodValidate)]
    struct Session {
        #[rod(
            str {
                length: 3..=12,
            },
            async_check = |t: &str| std::future::ready(t.starts_with("tok"))
        )]
        token: &'static &'static str,
    }

    // Sync and async checks both see the innermost `&str`.
    let ok = Session { token: &TOKEN };
    assert!(ok.validate().is_ok(), "{}", ok.validate().unwrap_err());
    assert!(block_on(ok.validate_async()).is_ok(), "{}", block_on(ok.validate_async()).unwrap_err());

    static BAD: &str = "zzz456";
    let bad = Session { token: &BAD };
    // Sync rule passes (length 6), async check rejects the prefix.
    assert!(bad.validate().is_ok());
    let errors = block_on(bad.validate_async()).unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(errors
        .iter()
        .any(|e| matches!(e, RodValidateError::CheckFailed(path) if *path == "token")), "{}", errors);
}

// ---------------------------------------------------------------------------
// async_check_all (multi-issue, may await)
// ---------------------------------------------------------------------------

/// One awaited closure reporting every problem it finds.
#[derive(RodValidate)]
struct AsyncMultiCheck {
    #[rod(
        String { length: 5..=12 },
        async_check_all = |u: &String| std::future::ready({
            let mut issues = Vec::new();
            if !u.starts_with("user_") {
                issues.push("must start with user_");
            }
            if u.chars().any(|c| c.is_ascii_uppercase()) {
                issues.push("must be lowercase");
            }
            issues
        })
    )]
    username: String,
}

#[test]
fn async_check_all_reports_every_issue() {
    let user = AsyncMultiCheck {
        username: "BadHandle99".to_string(), // length ok; both async issues fire
    };
    // Sync rules pass...
    assert!(user.validate_all().is_ok(), "{}", user.validate_all().unwrap_err());
    // ...the awaited closure yields BOTH issues.
    let errors = block_on(user.validate_async()).unwrap_err();
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
}

#[test]
fn async_check_all_empty_vec_passes() {
    let user = AsyncMultiCheck {
        username: "user_kim".to_string(),
    };
    assert!(block_on(user.validate_async()).is_ok());
}

#[test]
fn async_check_all_combines_with_sync_rules() {
    // "kit" fails the sync length rule AND both async issues.
    let user = AsyncMultiCheck {
        username: "Kit".to_string(),
    };
    let errors = block_on(user.validate_async()).unwrap_err();
    assert_eq!(errors.len(), 3, "{}", errors);
    assert!(errors.iter().any(|e| matches!(e, RodValidateError::String(_))));
    assert_eq!(errors.iter().filter(|e| matches!(e, RodValidateError::UserDefined(_))).count(), 2);
}

#[test]
fn message_applies_to_async_check_all() {
    #[derive(RodValidate)]
    struct Reserved {
        #[rod(
            String { length: 2..=10 },
            message: "username is reserved",
            async_check_all = |u: &String| std::future::ready({
                if u.eq_ignore_ascii_case("root") { vec!["ignored"] } else { vec![] }
            })
        )]
        username: String,
    }

    let user = Reserved { username: "ROOT".to_string() };
    let errors = block_on(user.validate_async()).unwrap_err();
    assert_eq!(errors.len(), 1, "{}", errors);
    assert!(errors
        .iter()
        .any(|e| matches!(e, RodValidateError::UserDefined(msg) if msg == "username is reserved")), "{}", errors);
}

#[test]
fn enum_async_check_all_only_active_variant() {
    #[derive(RodValidate)]
    enum Payment {
        Card {
            #[rod(
                String { length: 4 },
                async_check_all = |pan: &String| std::future::ready({
                    let mut issues = Vec::new();
                    if !pan.chars().all(|c| c.is_ascii_digit()) { issues.push("digits only"); }
                    if !pan.starts_with("4") { issues.push("visa expected"); }
                    issues
                })
            )]
            number: String,
        },
        Cash,
    }

    // Both issues fire on the active variant.
    let bad = Payment::Card { number: "12ab".to_string() };
    let errors = block_on(bad.validate_async()).unwrap_err();
    assert_eq!(errors.len(), 2, "{}", errors);

    // Unit variant: no async checks to run.
    assert!(block_on(Payment::Cash.validate_async()).is_ok());
}

#[test]
fn async_check_all_conflicts_with_async_check() {
    // async_check + async_check_all on one field is a compile error,
    // asserted by the compile_fail doctest (proc-macro aborts fire during
    // compilation, not at runtime).
}
