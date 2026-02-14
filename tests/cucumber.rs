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
    match std::env::var("CUCUMBER_MODE").as_deref() {
        Ok("in-process") => {
            InProcessWorld::run("features/").await;
        }
        _ => {
            if has_compgen {
                FullStackWorld::run("features/").await;
            } else {
                FullStackWorld::cucumber()
                    .filter_run("features/", |_, _, sc| {
                        !sc.tags.iter().any(|t| t == "bash_completion")
                    })
                    .await;
            }
        }
    }
}
