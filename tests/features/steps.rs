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

#[given(regex = r#"^I mark the yak "(.+)" as done$"#)]
async fn done_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.done_yak(&yak_name)
}

#[given(regex = r#"^I mark the yak "(.+)" as done$"#)]
async fn done_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.done_yak(&yak_name)
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

#[when(regex = r#"^I list the yaks in "(.+)" format$"#)]
async fn list_yaks_format_full_stack(world: &mut FullStackWorld, format: String) -> Result<()> {
    world.list_yaks_with_format(&format)
}

#[when(regex = r#"^I list the yaks in "(.+)" format$"#)]
async fn list_yaks_format_in_process(world: &mut InProcessWorld, format: String) -> Result<()> {
    world.list_yaks_with_format(&format)
}

#[when(regex = r#"^I list the yaks in "(.+)" format filtering by "(.+)"$"#)]
async fn list_yaks_format_filter_full_stack(
    world: &mut FullStackWorld,
    format: String,
    only: String,
) -> Result<()> {
    world.list_yaks_with_format_and_filter(&format, &only)
}

#[when(regex = r#"^I list the yaks in "(.+)" format filtering by "(.+)"$"#)]
async fn list_yaks_format_filter_in_process(
    world: &mut InProcessWorld,
    format: String,
    only: String,
) -> Result<()> {
    world.list_yaks_with_format_and_filter(&format, &only)
}

#[when(regex = r#"^I add the yak "(.+)"$"#)]
async fn when_add_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.add_yak(&yak_name)
}

#[when(regex = r#"^I add the yak "(.+)"$"#)]
async fn when_add_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.add_yak(&yak_name)
}

#[when(regex = r#"^there should be (\d+) yaks?$"#)]
async fn yak_count_full_stack(world: &mut FullStackWorld, expected: usize) -> Result<()> {
    check_yak_count(world, expected)
}

#[when(regex = r#"^there should be (\d+) yaks?$"#)]
async fn yak_count_in_process(world: &mut InProcessWorld, expected: usize) -> Result<()> {
    check_yak_count(world, expected)
}

#[when(regex = r#"^I try to add the yak "(.+)"$"#)]
async fn try_add_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.try_add_yak(&yak_name)
}

#[when(regex = r#"^I try to add the yak "(.+)"$"#)]
async fn try_add_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.try_add_yak(&yak_name)
}

#[when(regex = r#"^I remove the yak "(.+)"$"#)]
async fn remove_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.remove_yak(&yak_name)
}

#[when(regex = r#"^I remove the yak "(.+)"$"#)]
async fn remove_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.remove_yak(&yak_name)
}

#[when(regex = r#"^I try to remove the yak "(.+)"$"#)]
async fn try_remove_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.try_remove_yak(&yak_name)
}

#[when(regex = r#"^I try to remove the yak "(.+)"$"#)]
async fn try_remove_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.try_remove_yak(&yak_name)
}

// ============================================================================
// Then steps
// ============================================================================

#[then(expr = "the command should fail")]
async fn command_fails_full_stack(world: &mut FullStackWorld) -> Result<()> {
    check_command_fails(world)
}

#[then(expr = "the command should fail")]
async fn command_fails_in_process(world: &mut InProcessWorld) -> Result<()> {
    check_command_fails(world)
}

#[then(regex = r#"^the error should contain "(.+)"$"#)]
async fn error_contains_full_stack(world: &mut FullStackWorld, expected: String) -> Result<()> {
    check_error_contains(world, &expected)
}

#[then(regex = r#"^the error should contain "(.+)"$"#)]
async fn error_contains_in_process(world: &mut InProcessWorld, expected: String) -> Result<()> {
    check_error_contains(world, &expected)
}

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

#[then(expr = "the output should be empty")]
async fn output_should_be_empty_full_stack(world: &mut FullStackWorld) -> Result<()> {
    check_empty_output(world)
}

#[then(expr = "the output should be empty")]
async fn output_should_be_empty_in_process(world: &mut InProcessWorld) -> Result<()> {
    check_empty_output(world)
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

fn check_yak_count<W: TestWorld>(world: &mut W, expected: usize) -> Result<()> {
    world.list_yaks_with_format("plain")?;
    let output = world.get_output();
    let actual = output.trim().lines().filter(|l| !l.is_empty()).count();

    if actual != expected {
        anyhow::bail!("Expected {} yak(s), but found {}", expected, actual);
    }

    Ok(())
}

fn check_command_fails<W: TestWorld>(world: &W) -> Result<()> {
    if world.get_exit_code() == 0 {
        anyhow::bail!("Expected command to fail, but it succeeded");
    }
    Ok(())
}

fn check_error_contains<W: TestWorld>(world: &W, expected: &str) -> Result<()> {
    let error = world.get_error();
    if !error.contains(expected) {
        anyhow::bail!(
            "Expected error to contain '{}', but got: '{}'",
            expected,
            error
        );
    }
    Ok(())
}

fn check_empty_output<W: TestWorld>(world: &W) -> Result<()> {
    let output = world.get_output();
    let actual = output.trim();

    if !actual.is_empty() {
        anyhow::bail!("\nExpected empty output\n\nActual:\n{}", actual);
    }

    Ok(())
}
