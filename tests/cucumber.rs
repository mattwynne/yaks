mod features;

use cucumber::World as _;
use features::{full_stack_world::FullStackWorld, in_process_world::InProcessWorld};

#[tokio::test]
async fn run_all_features() {
    // Choose World implementation based on CUCUMBER_MODE env var
    match std::env::var("CUCUMBER_MODE").as_deref() {
        Ok("in-process") => {
            InProcessWorld::run("features/").await;
        }
        _ => {
            FullStackWorld::run("features/").await;
        }
    }
}
