mod features;

use cucumber::World as _;
use features::{full_stack_world::FullStackWorld, in_process_world::InProcessWorld};

fn has_bash_completion_support() -> bool {
    std::process::Command::new("bash")
        .args(["-c", "type compgen"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tokio::test]
async fn run_all_features() {
    let has_compgen = has_bash_completion_support();

    // Choose World implementation based on CUCUMBER_MODE env var
    // Default is in-process (fast, ~1s). Use CUCUMBER_MODE=fullstack for
    // full integration tests that spawn the yx binary (~39s).
    match std::env::var("CUCUMBER_MODE").as_deref() {
        Ok("fullstack") => {
            FullStackWorld::cucumber()
                .filter_run_and_exit("features/", move |_, rule, sc| {
                    let wip = sc.tags.iter().any(|t| t == "wip")
                        || rule.is_some_and(|r| r.tags.iter().any(|t| t == "wip"));
                    !wip && (has_compgen || !sc.tags.iter().any(|t| t == "bash_completion"))
                })
                .await;
        }
        _ => {
            InProcessWorld::cucumber()
                .filter_run_and_exit("features/", |_, rule, sc| {
                    let wip = sc.tags.iter().any(|t| t == "wip")
                        || rule.is_some_and(|r| r.tags.iter().any(|t| t == "wip"));
                    let fullstack = sc.tags.iter().any(|t| t == "fullstack")
                        || rule.is_some_and(|r| r.tags.iter().any(|t| t == "fullstack"));
                    !wip && !fullstack
                })
                .await;
        }
    }
}
