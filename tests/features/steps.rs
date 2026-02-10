// Step definitions using TestWorld trait
//
// These steps work with both FullStackWorld and InProcessWorld
// through the TestWorld trait interface.

use anyhow::{Context, Result};
use cucumber::{given, then, when};

use super::full_stack_world::FullStackWorld;
use super::in_process_world::InProcessWorld;
use super::test_world::{strip_ansi_codes, TestWorld};

// ============================================================================
// Given steps
// ============================================================================

#[given(expr = "I have a clean git repository")]
async fn clean_git_repo_full_stack(world: &mut FullStackWorld) -> Result<()> {
    world.init_git()
}

#[given(expr = "I have a clean git repository")]
async fn clean_git_repo_in_process(_world: &mut InProcessWorld) -> Result<()> {
    // No git needed in in-process mode
    Ok(())
}

#[given(regex = r#"^I add the yak "(.+)"$"#)]
async fn add_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.add_yak(&yak_name)
}

#[given(regex = r#"^I add the yak "(.+)"$"#)]
async fn add_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.add_yak(&yak_name)
}

// ============================================================================
// When steps
// ============================================================================

#[when(expr = "I list the yaks")]
async fn list_yaks_full_stack(world: &mut FullStackWorld) -> Result<()> {
    world.list_yaks()
}

#[when(expr = "I list the yaks")]
async fn list_yaks_in_process(world: &mut InProcessWorld) -> Result<()> {
    world.list_yaks()
}

// ============================================================================
// Then steps
// ============================================================================

#[then(expr = "the output should be:")]
async fn output_should_be_full_stack(
    world: &mut FullStackWorld,
    step: &cucumber::gherkin::Step,
) -> Result<()> {
    check_output(world, step)
}

#[then(expr = "the output should be:")]
async fn output_should_be_in_process(
    world: &mut InProcessWorld,
    step: &cucumber::gherkin::Step,
) -> Result<()> {
    check_output(world, step)
}

// ============================================================================
// Helper functions
// ============================================================================

fn check_output<W: TestWorld>(world: &W, step: &cucumber::gherkin::Step) -> Result<()> {
    let expected = step
        .docstring
        .as_ref()
        .context("Expected docstring in step")?;

    let expected_text = expected.trim();
    let output = world.get_output();
    let actual = output.trim();
    let actual_no_ansi = strip_ansi_codes(actual);

    if actual_no_ansi != expected_text {
        anyhow::bail!(
            "\nExpected:\n{}\n\nActual:\n{}",
            expected_text,
            actual_no_ansi
        );
    }

    Ok(())
}
