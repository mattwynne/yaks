// Step definitions using TestWorld trait
//
// These steps work with both FullStackWorld and InProcessWorld
// through the TestWorld trait interface.
//
// The `both_worlds!` macro eliminates duplication by generating
// step registrations for both world types from a single definition.
// Each shared step has one implementation function that works with
// `&mut dyn TestWorld`, and the macro creates thin wrappers for
// both concrete world types.

use anyhow::{Context, Result};
use cucumber::{given, then, when};

use super::full_stack_world::FullStackWorld;
use super::in_process_world::InProcessWorld;
use super::test_world::{strip_ansi_codes, TestWorld};
use yx::application::{AddYak, EditContext, ListYaks, MoveYak, RemoveYak, SetState, ShowContext};

// ============================================================================
// Macro to eliminate step definition duplication
// ============================================================================
//
// Cucumber-rs proc macros require step functions to have a concrete World
// type parameter. Since we run tests against both FullStackWorld and
// InProcessWorld, every shared step would need two identical definitions.
//
// This macro generates two thin wrappers (one per world type) that delegate
// to a shared implementation function taking `&mut dyn TestWorld`.

macro_rules! both_worlds {
    // given — no extra params
    (given($($attr:tt)*) fn $fs_name:ident / $ip_name:ident () -> $impl_fn:ident) => {
        #[given($($attr)*)]
        async fn $fs_name(world: &mut FullStackWorld) -> Result<()> {
            $impl_fn(world)
        }

        #[given($($attr)*)]
        async fn $ip_name(world: &mut InProcessWorld) -> Result<()> {
            $impl_fn(world)
        }
    };
    // given — with params
    (given($($attr:tt)*) fn $fs_name:ident / $ip_name:ident ($($p:ident : $t:ident),+) -> $impl_fn:ident) => {
        #[given($($attr)*)]
        async fn $fs_name(world: &mut FullStackWorld, $($p : $t),+) -> Result<()> {
            $impl_fn(world, $($p),+)
        }

        #[given($($attr)*)]
        async fn $ip_name(world: &mut InProcessWorld, $($p : $t),+) -> Result<()> {
            $impl_fn(world, $($p),+)
        }
    };

    // when — no extra params
    (when($($attr:tt)*) fn $fs_name:ident / $ip_name:ident () -> $impl_fn:ident) => {
        #[when($($attr)*)]
        async fn $fs_name(world: &mut FullStackWorld) -> Result<()> {
            $impl_fn(world)
        }

        #[when($($attr)*)]
        async fn $ip_name(world: &mut InProcessWorld) -> Result<()> {
            $impl_fn(world)
        }
    };
    // when — with params
    (when($($attr:tt)*) fn $fs_name:ident / $ip_name:ident ($($p:ident : $t:ident),+) -> $impl_fn:ident) => {
        #[when($($attr)*)]
        async fn $fs_name(world: &mut FullStackWorld, $($p : $t),+) -> Result<()> {
            $impl_fn(world, $($p),+)
        }

        #[when($($attr)*)]
        async fn $ip_name(world: &mut InProcessWorld, $($p : $t),+) -> Result<()> {
            $impl_fn(world, $($p),+)
        }
    };

    // then — no extra params
    (then($($attr:tt)*) fn $fs_name:ident / $ip_name:ident () -> $impl_fn:ident) => {
        #[then($($attr)*)]
        async fn $fs_name(world: &mut FullStackWorld) -> Result<()> {
            $impl_fn(world)
        }

        #[then($($attr)*)]
        async fn $ip_name(world: &mut InProcessWorld) -> Result<()> {
            $impl_fn(world)
        }
    };
    // then — with params
    (then($($attr:tt)*) fn $fs_name:ident / $ip_name:ident ($($p:ident : $t:ident),+) -> $impl_fn:ident) => {
        #[then($($attr)*)]
        async fn $fs_name(world: &mut FullStackWorld, $($p : $t),+) -> Result<()> {
            $impl_fn(world, $($p),+)
        }

        #[then($($attr)*)]
        async fn $ip_name(world: &mut InProcessWorld, $($p : $t),+) -> Result<()> {
            $impl_fn(world, $($p),+)
        }
    };

    // then_docstring — step gets a &Step param
    (then_docstring($($attr:tt)*) fn $fs_name:ident / $ip_name:ident () -> $impl_fn:ident) => {
        #[then($($attr)*)]
        async fn $fs_name(world: &mut FullStackWorld, step: &cucumber::gherkin::Step) -> Result<()> {
            $impl_fn(world, step)
        }

        #[then($($attr)*)]
        async fn $ip_name(world: &mut InProcessWorld, step: &cucumber::gherkin::Step) -> Result<()> {
            $impl_fn(world, step)
        }
    };
}

// ============================================================================
// Shared step implementations (work with any TestWorld)
// ============================================================================

fn impl_add_yak(world: &mut dyn TestWorld, yak_name: String) -> Result<()> {
    world.add_yak(&yak_name)
}

fn impl_add_yak_under(world: &mut dyn TestWorld, yak_name: String, parent: String) -> Result<()> {
    world.add_yak_under(&yak_name, &parent)
}

fn impl_done_yak(world: &mut dyn TestWorld, yak_name: String) -> Result<()> {
    world.done_yak(&yak_name)
}

fn impl_list_yaks(world: &mut dyn TestWorld) -> Result<()> {
    world.list_yaks()
}

fn impl_list_yaks_format(world: &mut dyn TestWorld, format: String) -> Result<()> {
    world.list_yaks_with_format(&format)
}

fn impl_list_yaks_format_filter(
    world: &mut dyn TestWorld,
    format: String,
    only: String,
) -> Result<()> {
    world.list_yaks_with_format_and_filter(&format, &only)
}

fn impl_list_yaks_json(world: &mut dyn TestWorld) -> Result<()> {
    world.list_yaks_json()
}

fn impl_try_list_yaks_format(world: &mut dyn TestWorld, format: String) -> Result<()> {
    world.try_list_yaks_with_format(&format)
}

fn impl_try_list_yaks_filter(world: &mut dyn TestWorld, only: String) -> Result<()> {
    world.try_list_yaks_with_filter(&only)
}

fn impl_yak_count(world: &mut dyn TestWorld, expected: usize) -> Result<()> {
    check_yak_count(world, expected)
}

fn impl_try_add_yak(world: &mut dyn TestWorld, yak_name: String) -> Result<()> {
    world.try_add_yak(&yak_name)
}

fn impl_try_add_yak_under(
    world: &mut dyn TestWorld,
    yak_name: String,
    parent: String,
) -> Result<()> {
    world.try_add_yak_under(&yak_name, &parent)
}

fn impl_remove_yak(world: &mut dyn TestWorld, yak_name: String) -> Result<()> {
    world.remove_yak(&yak_name)
}

fn impl_remove_yak_recursive(world: &mut dyn TestWorld, yak_name: String) -> Result<()> {
    world.remove_yak_recursive(&yak_name)
}

fn impl_try_remove_yak(world: &mut dyn TestWorld, yak_name: String) -> Result<()> {
    world.try_remove_yak(&yak_name)
}

fn impl_done_yak_when(world: &mut dyn TestWorld, yak_name: String) -> Result<()> {
    world.done_yak(&yak_name)
}

fn impl_try_done_yak(world: &mut dyn TestWorld, yak_name: String) -> Result<()> {
    world.try_done_yak(&yak_name)
}

fn impl_done_yak_recursive(world: &mut dyn TestWorld, yak_name: String) -> Result<()> {
    world.done_yak_recursive(&yak_name)
}

fn impl_set_context(world: &mut dyn TestWorld, name: String, content: String) -> Result<()> {
    world.set_context(&name, &content)
}

fn impl_show_context(world: &mut dyn TestWorld, name: String) -> Result<()> {
    world.show_context(&name)
}

fn impl_prune_done_yaks(world: &mut dyn TestWorld) -> Result<()> {
    world.prune_yaks()
}

fn impl_set_state(world: &mut dyn TestWorld, name: String, state: String) -> Result<()> {
    world.set_state(&name, &state)
}

fn impl_try_set_state(world: &mut dyn TestWorld, name: String, state: String) -> Result<()> {
    world.try_set_state(&name, &state)
}

fn impl_start_yak(world: &mut dyn TestWorld, name: String) -> Result<()> {
    world.start_yak(&name)
}

fn impl_move_yak_under(world: &mut dyn TestWorld, name: String, parent: String) -> Result<()> {
    world.move_yak_under(&name, &parent)
}

fn impl_move_yak_to_root(world: &mut dyn TestWorld, name: String) -> Result<()> {
    world.move_yak_to_root(&name)
}

fn impl_try_move_both_flags(world: &mut dyn TestWorld, name: String, parent: String) -> Result<()> {
    world.try_move_yak_under_and_to_root(&name, &parent)
}

fn impl_try_move_no_flags(world: &mut dyn TestWorld, name: String) -> Result<()> {
    world.try_move_yak_no_flags(&name)
}

fn impl_rename_yak(world: &mut dyn TestWorld, from: String, to: String) -> Result<()> {
    world.rename_yak(&from, &to)
}

fn impl_try_rename_yak(world: &mut dyn TestWorld, from: String, to: String) -> Result<()> {
    world.try_rename_yak(&from, &to)
}

fn impl_set_field(
    world: &mut dyn TestWorld,
    field: String,
    name: String,
    content: String,
) -> Result<()> {
    world.set_field(&name, &field, &content)
}

fn impl_try_set_field(
    world: &mut dyn TestWorld,
    field: String,
    name: String,
    content: String,
) -> Result<()> {
    world.try_set_field(&name, &field, &content)
}

fn impl_show_field(world: &mut dyn TestWorld, field: String, name: String) -> Result<()> {
    world.show_field(&name, &field)
}

fn impl_add_yak_with_state(
    world: &mut dyn TestWorld,
    yak_name: String,
    state: String,
) -> Result<()> {
    world.add_yak_with_state(&yak_name, &state)
}

fn impl_add_yak_with_context(
    world: &mut dyn TestWorld,
    yak_name: String,
    context: String,
) -> Result<()> {
    world.add_yak_with_context(&yak_name, &context)
}

fn impl_add_yak_with_id(world: &mut dyn TestWorld, yak_name: String, id: String) -> Result<()> {
    world.add_yak_with_id(&yak_name, &id)
}

fn impl_add_yak_with_field(
    world: &mut dyn TestWorld,
    yak_name: String,
    key: String,
    value: String,
) -> Result<()> {
    world.add_yak_with_field(&yak_name, &key, &value)
}

fn impl_tag_yak(world: &mut dyn TestWorld, name: String, tag: String) -> Result<()> {
    world.add_tags(&name, vec![tag])
}

fn impl_tag_yak_multi(
    world: &mut dyn TestWorld,
    name: String,
    tag1: String,
    tag2: String,
) -> Result<()> {
    world.add_tags(&name, vec![tag1, tag2])
}

fn impl_remove_tag(world: &mut dyn TestWorld, tag: String, name: String) -> Result<()> {
    world.remove_tags(&name, vec![tag])
}

fn impl_list_tags(world: &mut dyn TestWorld, name: String) -> Result<()> {
    world.list_tags(&name)
}

fn impl_command_fails(world: &dyn TestWorld) -> Result<()> {
    check_command_fails(world)
}

fn impl_error_contains(world: &dyn TestWorld, expected: String) -> Result<()> {
    check_error_contains(world, &expected)
}

fn impl_output_should_be(world: &dyn TestWorld, step: &cucumber::gherkin::Step) -> Result<()> {
    check_output(world, step)
}

fn impl_output_empty(world: &dyn TestWorld) -> Result<()> {
    check_empty_output(world)
}

fn impl_should_succeed(world: &dyn TestWorld) -> Result<()> {
    check_should_succeed(world)
}

fn impl_output_includes(world: &dyn TestWorld, expected: String) -> Result<()> {
    check_output_includes(world, &expected)
}

fn impl_output_not_includes(world: &dyn TestWorld, expected: String) -> Result<()> {
    check_output_not_includes(world, &expected)
}

fn impl_line_of_output_includes(
    world: &dyn TestWorld,
    line_num: usize,
    expected: String,
) -> Result<()> {
    check_line_of_output_includes(world, line_num, &expected)
}

// ============================================================================
// Shared step registrations (both FullStackWorld and InProcessWorld)
// ============================================================================

// -- Given steps --

both_worlds!(given(regex = r#"^I add the yak "([^"]+)"$"#)
    fn given_add_yak_fs / given_add_yak_ip (yak_name: String) -> impl_add_yak);

both_worlds!(given(regex = r#"^I add the yak "([^"]+)" under "([^"]+)"$"#)
    fn given_add_yak_under_fs / given_add_yak_under_ip (yak_name: String, parent: String) -> impl_add_yak_under);

both_worlds!(given(regex = r#"^I mark the yak "(.+)" as done$"#)
    fn given_done_yak_fs / given_done_yak_ip (yak_name: String) -> impl_done_yak);

// -- When steps --

both_worlds!(when(expr = "I list the yaks")
    fn when_list_yaks_fs / when_list_yaks_ip () -> impl_list_yaks);

both_worlds!(when(regex = r#"^I list the yaks in "(.+)" format$"#)
    fn when_list_yaks_format_fs / when_list_yaks_format_ip (format: String) -> impl_list_yaks_format);

both_worlds!(when(regex = r#"^I list the yaks in "(.+)" format filtering by "(.+)"$"#)
    fn when_list_yaks_format_filter_fs / when_list_yaks_format_filter_ip (format: String, only: String) -> impl_list_yaks_format_filter);

both_worlds!(when(expr = "I list the yaks as json")
    fn when_list_yaks_json_fs / when_list_yaks_json_ip () -> impl_list_yaks_json);

both_worlds!(when(regex = r#"^I try to list the yaks in "(.+)" format$"#)
    fn when_try_list_yaks_format_fs / when_try_list_yaks_format_ip (format: String) -> impl_try_list_yaks_format);

both_worlds!(when(regex = r#"^I try to list the yaks filtering by "(.+)"$"#)
    fn when_try_list_yaks_filter_fs / when_try_list_yaks_filter_ip (only: String) -> impl_try_list_yaks_filter);

both_worlds!(when(regex = r#"^I add the yak "([^"]+)"$"#)
    fn when_add_yak_fs / when_add_yak_ip (yak_name: String) -> impl_add_yak);

both_worlds!(when(regex = r#"^I add the yak "([^"]+)" under "([^"]+)"$"#)
    fn when_add_yak_under_fs / when_add_yak_under_ip (yak_name: String, parent: String) -> impl_add_yak_under);

both_worlds!(when(regex = r#"^there should be (\d+) yaks?$"#)
    fn when_yak_count_fs / when_yak_count_ip (expected: usize) -> impl_yak_count);

both_worlds!(when(regex = r#"^I try to add the yak "([^"]+)"$"#)
    fn when_try_add_yak_fs / when_try_add_yak_ip (yak_name: String) -> impl_try_add_yak);

both_worlds!(when(regex = r#"^I try to add the yak "([^"]+)" under "([^"]+)"$"#)
    fn when_try_add_yak_under_fs / when_try_add_yak_under_ip (yak_name: String, parent: String) -> impl_try_add_yak_under);

both_worlds!(when(regex = r#"^I remove the yak "(.+)"$"#)
    fn when_remove_yak_fs / when_remove_yak_ip (yak_name: String) -> impl_remove_yak);

both_worlds!(when(regex = r#"^I remove the yak "(.+)" recursively$"#)
    fn when_remove_yak_recursive_fs / when_remove_yak_recursive_ip (yak_name: String) -> impl_remove_yak_recursive);

both_worlds!(when(regex = r#"^I try to remove the yak "(.+)"$"#)
    fn when_try_remove_yak_fs / when_try_remove_yak_ip (yak_name: String) -> impl_try_remove_yak);

both_worlds!(when(regex = r#"^I mark the yak "(.+)" as done$"#)
    fn when_done_yak_fs / when_done_yak_ip (yak_name: String) -> impl_done_yak_when);

both_worlds!(when(regex = r#"^I try to mark the yak "(.+)" as done$"#)
    fn when_try_done_yak_fs / when_try_done_yak_ip (yak_name: String) -> impl_try_done_yak);

both_worlds!(when(regex = r#"^I mark the yak "(.+)" as done recursively$"#)
    fn when_done_yak_recursive_fs / when_done_yak_recursive_ip (yak_name: String) -> impl_done_yak_recursive);

both_worlds!(when(regex = r#"^I set the context of "(.+)" to "(.+)"$"#)
    fn when_set_context_fs / when_set_context_ip (name: String, content: String) -> impl_set_context);

both_worlds!(when(regex = r#"^I show the context of "(.+)"$"#)
    fn when_show_context_fs / when_show_context_ip (name: String) -> impl_show_context);

both_worlds!(when(expr = "I prune done yaks")
    fn when_prune_done_yaks_fs / when_prune_done_yaks_ip () -> impl_prune_done_yaks);

both_worlds!(when(regex = r#"^I set the state of "(.+)" to "(.+)"$"#)
    fn when_set_state_fs / when_set_state_ip (name: String, state: String) -> impl_set_state);

both_worlds!(when(regex = r#"^I try to set the state of "(.+)" to "(.+)"$"#)
    fn when_try_set_state_fs / when_try_set_state_ip (name: String, state: String) -> impl_try_set_state);

both_worlds!(when(regex = r#"^I start "(.+)"$"#)
    fn when_start_yak_fs / when_start_yak_ip (name: String) -> impl_start_yak);

both_worlds!(when(regex = r#"^I move the yak "(.+)" under "(.+)"$"#)
    fn when_move_yak_under_fs / when_move_yak_under_ip (name: String, parent: String) -> impl_move_yak_under);

both_worlds!(when(regex = r#"^I move the yak "(.+)" to root$"#)
    fn when_move_yak_to_root_fs / when_move_yak_to_root_ip (name: String) -> impl_move_yak_to_root);

both_worlds!(when(regex = r#"^I try to move the yak "(.+)" under "(.+)" to root$"#)
    fn when_try_move_both_flags_fs / when_try_move_both_flags_ip (name: String, parent: String) -> impl_try_move_both_flags);

both_worlds!(when(regex = r#"^I try to move the yak "(.+)" with no flags$"#)
    fn when_try_move_no_flags_fs / when_try_move_no_flags_ip (name: String) -> impl_try_move_no_flags);

fn impl_try_move_under(world: &mut dyn TestWorld, name: String, parent: String) -> Result<()> {
    world.try_move_yak_under(&name, &parent)
}

both_worlds!(when(regex = r#"^I try to move the yak "(.+)" under "(.+)"$"#)
    fn when_try_move_under_fs / when_try_move_under_ip (name: String, parent: String) -> impl_try_move_under);

both_worlds!(when(regex = r#"^I rename the yak "(.+)" to "(.+)"$"#)
    fn when_rename_yak_fs / when_rename_yak_ip (from: String, to: String) -> impl_rename_yak);

both_worlds!(when(regex = r#"^I try to rename the yak "(.+)" to "(.+)"$"#)
    fn when_try_rename_yak_fs / when_try_rename_yak_ip (from: String, to: String) -> impl_try_rename_yak);

both_worlds!(when(regex = r#"^I set the "(.+)" field of "(.+)" to "(.+)"$"#)
    fn when_set_field_fs / when_set_field_ip (field: String, name: String, content: String) -> impl_set_field);

both_worlds!(when(regex = r#"^I try to set the "(.+)" field of "(.+)" to "(.+)"$"#)
    fn when_try_set_field_fs / when_try_set_field_ip (field: String, name: String, content: String) -> impl_try_set_field);

both_worlds!(when(regex = r#"^I show the "(.+)" field of "(.+)"$"#)
    fn when_show_field_fs / when_show_field_ip (field: String, name: String) -> impl_show_field);

both_worlds!(when(regex = r#"^I add the yak "([^"]+)" with state "([^"]+)"$"#)
    fn when_add_yak_with_state_fs / when_add_yak_with_state_ip (yak_name: String, state: String) -> impl_add_yak_with_state);

both_worlds!(when(regex = r#"^I add the yak "([^"]+)" with context "([^"]+)"$"#)
    fn when_add_yak_with_context_fs / when_add_yak_with_context_ip (yak_name: String, context: String) -> impl_add_yak_with_context);

both_worlds!(when(regex = r#"^I add the yak "([^"]+)" with id "([^"]+)"$"#)
    fn when_add_yak_with_id_fs / when_add_yak_with_id_ip (yak_name: String, id: String) -> impl_add_yak_with_id);

both_worlds!(when(regex = r#"^I add the yak "([^"]+)" with field "([^"]+)" set to "([^"]+)"$"#)
    fn when_add_yak_with_field_fs / when_add_yak_with_field_ip (yak_name: String, key: String, value: String) -> impl_add_yak_with_field);

// -- Tag steps --

both_worlds!(when(regex = r#"^I tag "([^"]+)" with "([^"]+)"$"#)
    fn when_tag_yak_fs / when_tag_yak_ip (name: String, tag: String) -> impl_tag_yak);

both_worlds!(when(regex = r#"^I tag "(.+)" with "(.+)" and "(.+)"$"#)
    fn when_tag_yak_multi_fs / when_tag_yak_multi_ip (name: String, tag1: String, tag2: String) -> impl_tag_yak_multi);

both_worlds!(when(regex = r#"^I remove the tag "(.+)" from "(.+)"$"#)
    fn when_remove_tag_fs / when_remove_tag_ip (tag: String, name: String) -> impl_remove_tag);

both_worlds!(when(regex = r#"^I list tags on "(.+)"$"#)
    fn when_list_tags_fs / when_list_tags_ip (name: String) -> impl_list_tags);

// -- Then steps --

both_worlds!(then(expr = "the command should fail")
    fn then_command_fails_fs / then_command_fails_ip () -> impl_command_fails);

both_worlds!(then(regex = r#"^the error should contain "(.+)"$"#)
    fn then_error_contains_fs / then_error_contains_ip (expected: String) -> impl_error_contains);

both_worlds!(then_docstring(expr = "the output should be:")
    fn then_output_should_be_fs / then_output_should_be_ip () -> impl_output_should_be);

both_worlds!(then(expr = "the output should be empty")
    fn then_output_empty_fs / then_output_empty_ip () -> impl_output_empty);

both_worlds!(then(expr = "it should succeed")
    fn then_should_succeed_fs / then_should_succeed_ip () -> impl_should_succeed);

both_worlds!(then(regex = r#"^the output should include "(.+)"$"#)
    fn then_output_includes_fs / then_output_includes_ip (expected: String) -> impl_output_includes);

both_worlds!(then(regex = r#"^the output should not include "(.+)"$"#)
    fn then_output_not_includes_fs / then_output_not_includes_ip (expected: String) -> impl_output_not_includes);

both_worlds!(then(regex = r#"^line (\d+) of the output should include "(.+)"$"#)
    fn then_line_of_output_includes_fs / then_line_of_output_includes_ip (line_num: usize, expected: String) -> impl_line_of_output_includes);

// -- Multi-repo steps shared via matching methods on both worlds --

both_worlds!(given(regex = r#"^a bare git repository called ([\w-]+)$"#)
    fn given_bare_git_repo_fs / given_bare_git_repo_ip (name: String) -> impl_bare_git_repo);

both_worlds!(given(regex = r#"^a git clone of ([\w-]+) called ([\w-]+)$"#)
    fn given_git_clone_fs / given_git_clone_ip (origin: String, clone_name: String) -> impl_git_clone);

fn impl_bare_git_repo(world: &mut dyn TestWorld, name: String) -> Result<()> {
    world.create_bare_repo(&name)
}

fn impl_git_clone(world: &mut dyn TestWorld, origin: String, clone_name: String) -> Result<()> {
    world.create_clone(&origin, &clone_name)
}

// ============================================================================
// Steps where the two worlds differ: "I have a clean git repository"
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

// ============================================================================
// V1/V2 schema fixtures (full-stack only — frozen snapshots of old formats)
// ============================================================================

/// Create a yak directly in the git event store using the v1 schema format.
/// This is intentionally duplicated/inlined — it's a frozen snapshot of how
/// v1 works, so that when the production code evolves, this fixture still
/// creates the old format to prove migration works.
#[given(regex = r#"^a yak "(.+)" created with the v1 schema$"#)]
async fn v1_yak(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.init_git()?;
    let repo_path = world.default_repo_path();

    let state_oid = git_hash_object(repo_path, "todo")?;
    let context_oid = git_hash_object(repo_path, "")?;

    let yak_tree_input = format!(
        "100644 blob {}\tstate\n100644 blob {}\tcontext.md\n",
        state_oid, context_oid
    );
    let yak_tree_oid = git_mktree(repo_path, &yak_tree_input)?;

    let root_tree_input = format!("040000 tree {}\t{}\n", yak_tree_oid, yak_name);
    let root_tree_oid = git_mktree(repo_path, &root_tree_input)?;

    let message = format!("Added: \"{}\"", yak_name);
    let commit_oid = git_commit_tree(repo_path, &root_tree_oid, &message, None)?;
    git_update_ref(repo_path, "refs/notes/yaks", &commit_oid)?;

    let yak_dir = repo_path.join(&yak_name);
    std::fs::create_dir_all(&yak_dir).context("Failed to create yak directory")?;
    std::fs::write(yak_dir.join("state"), "todo").context("Failed to write state")?;
    std::fs::write(yak_dir.join("context.md"), "").context("Failed to write context.md")?;

    Ok(())
}

/// Create a yak directly in the git event store using the v2 schema format.
#[given(regex = r#"^a yak "(.+)" created with the v2 schema$"#)]
async fn v2_yak(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.init_git()?;
    let repo_path = world.default_repo_path();

    let state_oid = git_hash_object(repo_path, "todo")?;
    let context_oid = git_hash_object(repo_path, "")?;
    let version_oid = git_hash_object(repo_path, "2")?;

    let yak_tree_input = format!(
        "100644 blob {}\tstate\n100644 blob {}\tcontext.md\n",
        state_oid, context_oid
    );
    let yak_tree_oid = git_mktree(repo_path, &yak_tree_input)?;

    let root_tree_input = format!(
        "040000 tree {}\t{}\n100644 blob {}\t.schema-version\n",
        yak_tree_oid, yak_name, version_oid
    );
    let root_tree_oid = git_mktree(repo_path, &root_tree_input)?;

    let message = format!("Added: \"{}\"", yak_name);
    let commit_oid = git_commit_tree(repo_path, &root_tree_oid, &message, None)?;
    git_update_ref(repo_path, "refs/notes/yaks", &commit_oid)?;

    let yak_dir = repo_path.join(&yak_name);
    std::fs::create_dir_all(&yak_dir).context("Failed to create yak directory")?;
    std::fs::write(yak_dir.join("state"), "todo").context("Failed to write state")?;
    std::fs::write(yak_dir.join("context.md"), "").context("Failed to write context.md")?;

    Ok(())
}

// ============================================================================
// Corrupted git tree fixture (full-stack only)
// ============================================================================

#[given(regex = r#"^a corrupted git tree with duplicate entries for "(.+)"$"#)]
async fn corrupted_duplicate_tree(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.init_git()?;
    let repo_path = world.default_repo_path();

    let state_oid = git_hash_object(repo_path, "todo")?;
    let context_oid = git_hash_object(repo_path, "")?;
    let name_oid = git_hash_object(repo_path, &yak_name)?;

    let yak_tree_input = format!(
        "100644 blob {}\tstate\n100644 blob {}\tcontext.md\n100644 blob {}\tname\n",
        state_oid, context_oid, name_oid
    );
    let yak_tree_oid = git_mktree(repo_path, &yak_tree_input)?;

    let root_tree_input = format!(
        "040000 tree {}\t{}\n040000 tree {}\t{}-7bvf\n",
        yak_tree_oid, yak_name, yak_tree_oid, yak_name
    );
    let root_tree_oid = git_mktree(repo_path, &root_tree_input)?;

    let message = format!("Added: \"{}\"", yak_name);
    let commit_oid = git_commit_tree(repo_path, &root_tree_oid, &message, None)?;
    git_update_ref(repo_path, "refs/notes/yaks", &commit_oid)?;

    Ok(())
}

// -- Git plumbing helpers --

fn git_hash_object(repo_path: &std::path::Path, content: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["hash-object", "-w", "--stdin"])
        .current_dir(repo_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(content.as_bytes())?;
            child.wait_with_output()
        })
        .context("git hash-object failed")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_mktree(repo_path: &std::path::Path, input: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["mktree"])
        .current_dir(repo_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(input.as_bytes())?;
            child.wait_with_output()
        })
        .context("git mktree failed")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_commit_tree(
    repo_path: &std::path::Path,
    tree_oid: &str,
    message: &str,
    parent: Option<&str>,
) -> Result<String> {
    let mut args = vec!["commit-tree", tree_oid, "-m", message];
    if let Some(parent_oid) = parent {
        args.extend(["-p", parent_oid]);
    }
    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(repo_path)
        .output()
        .context("git commit-tree failed")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_update_ref(repo_path: &std::path::Path, ref_name: &str, oid: &str) -> Result<()> {
    let status = std::process::Command::new("git")
        .args(["update-ref", ref_name, oid])
        .current_dir(repo_path)
        .status()
        .context("git update-ref failed")?;
    if !status.success() {
        anyhow::bail!("git update-ref failed");
    }
    Ok(())
}

/// Stamp a schema version beyond what the current binary supports.
#[given(expr = "origin has been migrated beyond the current schema version")]
async fn origin_migrated_beyond(world: &mut FullStackWorld) -> Result<()> {
    let repo_path = world.repo_path("alice")?;
    let future_version = yx::adapters::event_store::migration::CURRENT_SCHEMA_VERSION + 1;
    let version_oid = git_hash_object(&repo_path, &future_version.to_string())?;

    let current_ref = std::process::Command::new("git")
        .args(["rev-parse", "refs/notes/yaks"])
        .current_dir(&repo_path)
        .output()
        .context("git rev-parse failed")?;
    let commit_oid = String::from_utf8_lossy(&current_ref.stdout)
        .trim()
        .to_string();

    let current_tree = std::process::Command::new("git")
        .args(["rev-parse", &format!("{}^{{tree}}", commit_oid)])
        .current_dir(&repo_path)
        .output()
        .context("git rev-parse tree failed")?;
    let tree_oid = String::from_utf8_lossy(&current_tree.stdout)
        .trim()
        .to_string();

    let ls_tree = std::process::Command::new("git")
        .args(["ls-tree", &tree_oid])
        .current_dir(&repo_path)
        .output()
        .context("git ls-tree failed")?;
    let ls_output = String::from_utf8_lossy(&ls_tree.stdout);

    let mut new_tree_input = String::new();
    for line in ls_output.lines() {
        if line.ends_with("\t.schema-version") {
            new_tree_input.push_str(&format!("100644 blob {}\t.schema-version\n", version_oid));
        } else {
            new_tree_input.push_str(line);
            new_tree_input.push('\n');
        }
    }
    let new_tree_oid = git_mktree(&repo_path, &new_tree_input)?;
    let new_commit_oid = git_commit_tree(
        &repo_path,
        &new_tree_oid,
        &format!("Schema version: {}", future_version),
        Some(&commit_oid),
    )?;
    git_update_ref(&repo_path, "refs/notes/yaks", &new_commit_oid)?;

    let push = std::process::Command::new("git")
        .args(["push", "origin", "+refs/notes/yaks:refs/notes/yaks"])
        .current_dir(&repo_path)
        .output()
        .context("git push failed")?;
    if !push.status.success() {
        anyhow::bail!(
            "Failed to push schema version to origin: {}",
            String::from_utf8_lossy(&push.stderr)
        );
    }

    Ok(())
}

// ============================================================================
// Full-stack-only steps (CLI behavior that can't be tested in-process)
// ============================================================================

#[given(expr = "a directory that is not a git repository")]
async fn dir_not_git_repo(world: &mut FullStackWorld) -> Result<()> {
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    world.override_dir = Some(temp_dir);
    Ok(())
}

#[given(expr = "a git repository without .yaks in .gitignore")]
async fn git_repo_without_gitignore(world: &mut FullStackWorld) -> Result<()> {
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let status = std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .current_dir(temp_dir.path())
        .status()
        .context("Failed to run git init")?;
    if !status.success() {
        anyhow::bail!("git init failed");
    }
    world.override_dir = Some(temp_dir);
    Ok(())
}

#[when(expr = "I try to list the yaks from this directory")]
async fn list_yaks_in_override_dir(world: &mut FullStackWorld) -> Result<()> {
    world.run_yx_in_override_dir(&["ls"])
}

#[when(expr = "I list the yaks with YX_SKIP_GIT_CHECKS set")]
async fn list_yaks_with_skip_git_checks(world: &mut FullStackWorld) -> Result<()> {
    world.run_yx_in_override_dir_skip_git_checks(&["ls"])
}

#[given(regex = r#"^a git repository with \.yaks gitignored and a yak called "([^"]+)"$"#)]
async fn git_repo_with_gitignored_yaks_and_yak(
    world: &mut FullStackWorld,
    yak_name: String,
) -> Result<()> {
    world.setup_git_repo_with_yak(&yak_name)?;
    world.create_subdir_in_git_repo("subdir")
}

#[given(regex = r#"^a git repository with YAK_PATH set and a yak called "([^"]+)"$"#)]
async fn git_repo_with_explicit_yak_path_and_yak(
    world: &mut FullStackWorld,
    yak_name: String,
) -> Result<()> {
    world.setup_git_repo_with_explicit_yak_path(&yak_name)?;
    world.create_subdir_in_git_repo("subdir")
}

#[given(expr = "YAK_PATH is set to a directory")]
async fn yak_path_set_to_directory(world: &mut FullStackWorld) -> Result<()> {
    let yak_path_temp_dir =
        tempfile::tempdir().context("Failed to create yak_path temp directory")?;
    world.explicit_yak_path = Some(yak_path_temp_dir);
    Ok(())
}

#[when(expr = "I list the yaks from a subdirectory of that repository")]
async fn list_yaks_from_subdir(world: &mut FullStackWorld) -> Result<()> {
    world.list_yaks_from_subdir()
}

#[when(expr = "I list the yaks from a subdirectory using YAK_PATH")]
async fn list_yaks_from_subdir_with_yak_path(world: &mut FullStackWorld) -> Result<()> {
    world.list_yaks_from_subdir_with_yak_path()
}

#[then(expr = "the command should succeed")]
async fn command_should_succeed(world: &mut FullStackWorld) -> Result<()> {
    check_should_succeed(world)
}

#[when(regex = r#"^I run yx (.+)$"#)]
async fn run_yx_raw_full_stack(world: &mut FullStackWorld, args: String) -> Result<()> {
    let parsed = shell_split(&args);
    let arg_vec: Vec<&str> = parsed.iter().map(|s| s.as_str()).collect();
    world.run_raw(&arg_vec)
}

#[when(regex = r#"^I try to run yx (.+)$"#)]
async fn try_run_yx_raw_full_stack(world: &mut FullStackWorld, args: String) -> Result<()> {
    let parsed = shell_split(&args);
    let arg_vec: Vec<&str> = parsed.iter().map(|s| s.as_str()).collect();
    world.run_raw(&arg_vec)
}

#[when(expr = "I invoke yx with no subcommand")]
async fn run_yx_no_args(world: &mut FullStackWorld) -> Result<()> {
    world.run_raw(&[])
}

#[when(regex = r#"^I add the yak "(.+)" with context "(.+)" from stdin$"#)]
async fn add_yak_with_stdin_full_stack(
    world: &mut FullStackWorld,
    yak_name: String,
    context: String,
) -> Result<()> {
    world.add_yak_with_stdin(&yak_name, &context)
}

#[when(regex = r#"^I add the yak "(.+)" with editor that writes "(.+)"$"#)]
async fn add_yak_with_editor_full_stack(
    world: &mut FullStackWorld,
    yak_name: String,
    content: String,
) -> Result<()> {
    world.add_yak_with_editor(&yak_name, &content)
}

#[when(regex = r#"^I set the context of "(.+)" from a file containing "(.+)"$"#)]
async fn set_context_from_file(
    world: &mut FullStackWorld,
    name: String,
    content: String,
) -> Result<()> {
    world.run_yx_with_file_stdin(&["context", &name], &content)
}

#[when(regex = r#"^I try to set the context of "(.+)" with empty stdin$"#)]
async fn try_set_context_empty_stdin(world: &mut FullStackWorld, name: String) -> Result<()> {
    world.run_yx_with_empty_stdin(&["context", &name])
}

#[when(regex = r#"^I try to set the "(.+)" field of "(.+)" with empty stdin$"#)]
async fn try_set_field_empty_stdin(
    world: &mut FullStackWorld,
    field: String,
    name: String,
) -> Result<()> {
    world.run_yx_with_empty_stdin(&["field", &name, &field])
}

#[when(regex = r#"^I try to show context of "(.+)" with piped input "(.+)"$"#)]
async fn try_show_context_with_piped_input(
    world: &mut FullStackWorld,
    name: String,
    content: String,
) -> Result<()> {
    world.run_yx_with_stdin_unchecked(&["context", "--show", &name], &content)
}

#[when(regex = r#"^I try to show "(.+)" field of "(.+)" with piped input "(.+)"$"#)]
async fn try_show_field_with_piped_input(
    world: &mut FullStackWorld,
    field: String,
    name: String,
    content: String,
) -> Result<()> {
    world.run_yx_with_stdin_unchecked(&["field", &name, &field, "--show"], &content)
}

#[when(regex = r#"^I edit the "(.+)" field of "(.+)" with editor that appends "(.+)"$"#)]
async fn edit_field_with_editor_append(
    world: &mut FullStackWorld,
    field: String,
    name: String,
    append_text: String,
) -> Result<()> {
    world.run_yx_with_editor(
        &["field", &name, &field, "--edit"],
        r#"printf '%s' "$APPEND_TEXT" >> "$1""#,
        &[("APPEND_TEXT", &append_text)],
    )
}

#[when(regex = r#"^I edit the context of "(.+)" with editor that appends "(.+)"$"#)]
async fn edit_context_with_editor_append(
    world: &mut FullStackWorld,
    name: String,
    append_text: String,
) -> Result<()> {
    world.run_yx_with_editor(
        &["context", &name, "--edit"],
        r#"printf '%s' "$APPEND_TEXT" >> "$1""#,
        &[("APPEND_TEXT", &append_text)],
    )
}

#[when(regex = r#"^I pipe "(.+)" and edit the context of "(.+)" with editor that appends "(.+)"$"#)]
async fn pipe_and_edit_context(
    world: &mut FullStackWorld,
    stdin_content: String,
    name: String,
    append_text: String,
) -> Result<()> {
    world.run_yx_with_stdin_and_editor(
        &["context", &name, "--edit"],
        &stdin_content,
        r#"printf '%s' "$APPEND_TEXT" >> "$1""#,
        &[("APPEND_TEXT", &append_text)],
    )
}

#[when(
    regex = r#"^I pipe "(.+)" and edit the "(.+)" field of "(.+)" with editor that appends "(.+)"$"#
)]
async fn pipe_and_edit_field(
    world: &mut FullStackWorld,
    stdin_content: String,
    field: String,
    name: String,
    append_text: String,
) -> Result<()> {
    world.run_yx_with_stdin_and_editor(
        &["field", &name, &field, "--edit"],
        &stdin_content,
        r#"printf '%s' "$APPEND_TEXT" >> "$1""#,
        &[("APPEND_TEXT", &append_text)],
    )
}

#[when(regex = r#"^I invoke bash completion for words: (.+)$"#)]
async fn invoke_bash_completion(world: &mut FullStackWorld, words_str: String) -> Result<()> {
    world.run_bash_completion(&words_str)
}

#[then(regex = r#"^the completions should include "(.+)"$"#)]
async fn completions_should_include(world: &mut FullStackWorld, expected: String) -> Result<()> {
    check_output_includes(world, &expected)
}

#[then(regex = r#"^the yak directory should be named "(.+)"$"#)]
async fn yak_directory_named(world: &mut FullStackWorld, slug: String) -> Result<()> {
    let dir = world.default_repo_path().join(&slug);
    if !dir.exists() {
        anyhow::bail!("Expected yak directory '{}' to exist at {:?}", slug, dir);
    }
    let marker = dir.join(".context.md");
    if !marker.exists() {
        anyhow::bail!(
            "Directory '{}' exists but does not contain .context.md",
            slug
        );
    }
    Ok(())
}

#[given(regex = r#"^a file "(.+)" exists in the yak directory$"#)]
async fn file_exists_in_yak_dir(world: &mut FullStackWorld, filename: String) -> Result<()> {
    let path = world.default_repo_path().join(&filename);
    std::fs::write(&path, "test content").context(format!("Failed to create {}", filename))
}

#[then(regex = r#"^the file "(.+)" should still exist in the yak directory$"#)]
async fn file_still_exists_in_yak_dir(world: &mut FullStackWorld, filename: String) -> Result<()> {
    let path = world.default_repo_path().join(&filename);
    if !path.exists() {
        anyhow::bail!("Expected file '{}' to still exist after reset", filename);
    }
    Ok(())
}

#[then(regex = r#"^the yak "(.+)" should have a "(.+)" file containing "(.+)"$"#)]
async fn yak_has_file_with_content(
    world: &mut FullStackWorld,
    yak_name: String,
    file_name: String,
    expected_content: String,
) -> Result<()> {
    let path = world.default_repo_path().join(&yak_name).join(&file_name);
    if !path.exists() {
        anyhow::bail!(
            "Expected file '{}' in yak '{}' directory, but it doesn't exist",
            file_name,
            yak_name
        );
    }
    let content = std::fs::read_to_string(&path)
        .context(format!("Failed to read {} for yak {}", file_name, yak_name))?;
    if content.trim() != expected_content {
        anyhow::bail!(
            "Expected '{}' file to contain '{}', got '{}'",
            file_name,
            expected_content,
            content.trim()
        );
    }
    Ok(())
}

#[then(regex = r#"^the yak "(.+)" should have an "(.+)" file$"#)]
async fn yak_has_file(
    world: &mut FullStackWorld,
    yak_name: String,
    file_name: String,
) -> Result<()> {
    let path = world.default_repo_path().join(&yak_name).join(&file_name);
    if !path.exists() {
        anyhow::bail!(
            "Expected file '{}' in yak '{}' directory, but it doesn't exist",
            file_name,
            yak_name
        );
    }
    let content = std::fs::read_to_string(&path)
        .context(format!("Failed to read {} for yak {}", file_name, yak_name))?;
    if content.trim().is_empty() {
        anyhow::bail!(
            "Expected '{}' file for yak '{}' to be non-empty",
            file_name,
            yak_name
        );
    }
    Ok(())
}

#[when(expr = "I reset the yaks")]
async fn reset_yaks_full_stack(world: &mut FullStackWorld) -> Result<()> {
    world.run_raw(&["reset"])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "yx reset failed:\nstdout: {}\nstderr: {}",
            world.get_output(),
            world.get_error()
        );
    }
    Ok(())
}

#[when(expr = "I reset the yaks from disk to git")]
async fn reset_yaks_git_from_disk(world: &mut FullStackWorld) -> Result<()> {
    world.run_raw(&["reset", "--git-from-disk", "--force"])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "yx reset --git-from-disk failed:\nstdout: {}\nstderr: {}",
            world.get_output(),
            world.get_error()
        );
    }
    Ok(())
}

#[when(expr = "I try to reset from disk")]
async fn try_reset_from_disk(world: &mut FullStackWorld) -> Result<()> {
    world.run_raw(&["reset", "--git-from-disk"])?;
    Ok(())
}

#[when(expr = "I reset from disk with --force")]
async fn reset_from_disk_with_force(world: &mut FullStackWorld) -> Result<()> {
    world.run_raw(&["reset", "--git-from-disk", "--force"])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "yx reset --git-from-disk --force failed:\nstdout: {}\nstderr: {}",
            world.get_output(),
            world.get_error()
        );
    }
    Ok(())
}

// ============================================================================
// Multi-repo steps with different implementations per world
// ============================================================================

#[given(regex = r#"^a git clone of ([\w-]+) via file URL called ([\w-]+)$"#)]
async fn git_clone_via_file_url(
    world: &mut FullStackWorld,
    origin: String,
    clone: String,
) -> Result<()> {
    world.create_clone_with_file_url(&origin, &clone)
}

#[given(regex = r#"^([\w-]+)'s origin remote is unreachable$"#)]
async fn origin_unreachable(world: &mut FullStackWorld, repo: String) -> Result<()> {
    world.make_origin_unreachable(&repo)
}

#[given(regex = r#"^a git worktree of ([\w-]+) called ([\w-]+)$"#)]
async fn git_worktree(world: &mut FullStackWorld, parent: String, worktree: String) -> Result<()> {
    world.create_worktree(&parent, &worktree)
}

#[given(regex = r#"^([\w-]+) (?:has|adds) a yak called "(.+)"$"#)]
#[when(regex = r#"^([\w-]+) (?:has|adds) a yak called "(.+)"$"#)]
async fn repo_has_yak(world: &mut FullStackWorld, repo: String, yak: String) -> Result<()> {
    world.run_yx_in_repo(&repo, &["add", &yak])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Failed to add yak '{}' in repo '{}':\nstdout: {}\nstderr: {}",
            yak,
            repo,
            world.get_output(),
            world.get_error()
        );
    }
    Ok(())
}

#[given(regex = r#"^([\w-]+) has a yak called "(.+)"$"#)]
async fn repo_has_yak_in_process(
    world: &mut InProcessWorld,
    repo: String,
    yak: String,
) -> Result<()> {
    world.execute_in_repo(&repo, |app| app.handle(AddYak::new(&yak)))
}

#[given(regex = r#"^([\w-]+) has set the state of "(.+)" to "(.+)"$"#)]
async fn repo_has_set_state(
    world: &mut FullStackWorld,
    repo: String,
    yak: String,
    state: String,
) -> Result<()> {
    world.run_yx_in_repo(&repo, &["state", &yak, &state])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Failed to set state of '{}' to '{}' in repo '{}':\nstderr: {}",
            yak,
            state,
            repo,
            world.get_error()
        );
    }
    Ok(())
}

#[given(regex = r#"^([\w-]+) has set the state of "(.+)" to "(.+)"$"#)]
async fn repo_has_set_state_in_process(
    world: &mut InProcessWorld,
    repo: String,
    yak: String,
    state: String,
) -> Result<()> {
    world.execute_in_repo(&repo, |app| app.handle(SetState::new(&yak, &state)))
}

#[given(regex = r#"^([\w-]+) has set the context of "(.+)" to "(.+)"$"#)]
async fn repo_has_set_context(
    world: &mut FullStackWorld,
    repo: String,
    yak: String,
    content: String,
) -> Result<()> {
    world.run_yx_in_repo_with_stdin(&repo, &["context", &yak], &content)?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Failed to set context of '{}' in repo '{}':\nstderr: {}",
            yak,
            repo,
            world.get_error()
        );
    }
    Ok(())
}

#[given(regex = r#"^([\w-]+) has set the context of "(.+)" to "(.+)"$"#)]
async fn repo_has_set_context_in_process(
    world: &mut InProcessWorld,
    repo: String,
    yak: String,
    content: String,
) -> Result<()> {
    world.set_input_in_repo(&repo, &content)?;
    world.execute_in_repo(&repo, |app| app.handle(EditContext::new(&yak)))
}

#[given(regex = r#"^([\w-]+) has removed the yak "(.+)"$"#)]
async fn repo_has_removed_yak(world: &mut FullStackWorld, repo: String, yak: String) -> Result<()> {
    world.run_yx_in_repo(&repo, &["rm", &yak])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Failed to remove yak '{}' in repo '{}':\nstderr: {}",
            yak,
            repo,
            world.get_error()
        );
    }
    Ok(())
}

#[given(regex = r#"^([\w-]+) has removed the yak "(.+)"$"#)]
async fn repo_has_removed_yak_in_process(
    world: &mut InProcessWorld,
    repo: String,
    yak: String,
) -> Result<()> {
    world.execute_in_repo(&repo, |app| app.handle(RemoveYak::new(&yak)))
}

#[given(regex = r#"^([\w-]+) has moved the yak "(.+)" under "(.+)"$"#)]
async fn repo_has_moved_yak_under(
    world: &mut FullStackWorld,
    repo: String,
    yak: String,
    parent: String,
) -> Result<()> {
    world.run_yx_in_repo(&repo, &["move", &yak, "--under", &parent])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Failed to move yak '{}' under '{}' in repo '{}':\nstderr: {}",
            yak,
            parent,
            repo,
            world.get_error()
        );
    }
    Ok(())
}

#[given(regex = r#"^([\w-]+) has moved the yak "(.+)" under "(.+)"$"#)]
async fn repo_has_moved_yak_under_in_process(
    world: &mut InProcessWorld,
    repo: String,
    yak: String,
    parent: String,
) -> Result<()> {
    world.execute_in_repo(&repo, |app| app.handle(MoveYak::under(&yak, &parent)))
}

#[given(regex = r#"^([\w-]+) has synced yaks$"#)]
async fn repo_has_synced(world: &mut FullStackWorld, repo: String) -> Result<()> {
    world.run_yx_in_repo(&repo, &["sync"])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Failed to sync yaks in repo '{}':\nstdout: {}\nstderr: {}",
            repo,
            world.get_output(),
            world.get_error()
        );
    }
    Ok(())
}

#[given(regex = r#"^([\w-]+) has synced yaks$"#)]
async fn repo_has_synced_in_process(world: &mut InProcessWorld, repo: String) -> Result<()> {
    world.sync_repo(&repo)
}

#[given(regex = r#"^([\w-]+) has compacted yaks$"#)]
async fn repo_has_compacted(world: &mut FullStackWorld, repo: String) -> Result<()> {
    world.run_yx_in_repo(&repo, &["compact", "--yes"])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Failed to compact yaks in repo '{}':\nstdout: {}\nstderr: {}",
            repo,
            world.get_output(),
            world.get_error()
        );
    }
    Ok(())
}

#[when(regex = r#"^([\w-]+) syncs yaks$"#)]
async fn repo_syncs_yaks(world: &mut FullStackWorld, repo: String) -> Result<()> {
    world.run_yx_in_repo(&repo, &["sync"])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Failed to sync yaks in repo '{}':\nstdout: {}\nstderr: {}",
            repo,
            world.get_output(),
            world.get_error()
        );
    }
    Ok(())
}

#[when(regex = r#"^([\w-]+) syncs yaks$"#)]
async fn repo_syncs_yaks_in_process(world: &mut InProcessWorld, repo: String) -> Result<()> {
    world.sync_repo(&repo)
}

#[when(regex = r#"^(\w+) tries to sync yaks$"#)]
async fn repo_tries_to_sync(world: &mut FullStackWorld, repo: String) -> Result<()> {
    world.run_yx_in_repo(&repo, &["sync"])?;
    Ok(())
}

#[then(regex = r#"^([\w-]+) has a "(.+)" ref$"#)]
async fn repo_has_ref(world: &mut FullStackWorld, repo: String, ref_name: String) -> Result<()> {
    world.run_git_in_repo(&repo, &["show-ref", &ref_name])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Expected repo '{}' to have ref '{}', but show-ref failed",
            repo,
            ref_name
        );
    }
    Ok(())
}

#[then(regex = r#"^([\w-]+) should have a yak called "(.+)"$"#)]
async fn repo_should_have_yak(world: &mut FullStackWorld, repo: String, yak: String) -> Result<()> {
    world.run_yx_in_repo(&repo, &["ls", "--format", "plain"])?;
    let output = world.get_output();
    if !output.lines().any(|line| line == yak) {
        anyhow::bail!(
            "Expected repo '{}' to have yak '{}', but output was:\n{}",
            repo,
            yak,
            output
        );
    }
    Ok(())
}

#[then(regex = r#"^([\w-]+) should have a yak called "(.+)"$"#)]
async fn repo_should_have_yak_in_process(
    world: &mut InProcessWorld,
    repo: String,
    yak: String,
) -> Result<()> {
    let yaks = world.list_yaks_in_repo(&repo)?;
    if !yaks.iter().any(|y| y.name.as_str() == yak) {
        let names: Vec<&str> = yaks.iter().map(|y| y.name.as_str()).collect();
        anyhow::bail!(
            "Expected repo '{}' to have yak '{}', but found: {:?}",
            repo,
            yak,
            names
        );
    }
    Ok(())
}

#[then(regex = r#"^([\w-]+) and ([\w-]+) both have the same yaks:$"#)]
async fn repos_have_same_yaks(
    world: &mut FullStackWorld,
    repo_a: String,
    repo_b: String,
    step: &cucumber::gherkin::Step,
) -> Result<()> {
    let expected = step
        .docstring
        .as_ref()
        .expect("step requires a docstring")
        .trim()
        .to_string();
    for repo in [&repo_a, &repo_b] {
        world.run_yx_in_repo(repo, &["ls", "--format", "pretty"])?;
        let output = world.get_output().trim().to_string();
        if output != expected {
            anyhow::bail!(
                "Expected repo '{}' to have yaks:\n{}\nbut got:\n{}",
                repo,
                expected,
                output
            );
        }
    }
    Ok(())
}

#[then(regex = r#"^([\w-]+) and ([\w-]+) both have the same yaks:$"#)]
async fn repos_have_same_yaks_in_process(
    world: &mut InProcessWorld,
    repo_a: String,
    repo_b: String,
    step: &cucumber::gherkin::Step,
) -> Result<()> {
    let expected = step
        .docstring
        .as_ref()
        .expect("step requires a docstring")
        .trim()
        .to_string();
    for repo in [&repo_a, &repo_b] {
        world.execute_in_repo(repo, |app| app.handle(ListYaks::new("pretty", None)))?;
        let output = world.get_repo_output(repo)?.trim().to_string();
        if output != expected {
            anyhow::bail!(
                "Expected repo '{}' to have yaks:\n{}\nbut got:\n{}",
                repo,
                expected,
                output
            );
        }
    }
    Ok(())
}

#[then(regex = r#"^([\w-]+) should have these yaks:$"#)]
async fn repo_should_have_yaks(
    world: &mut FullStackWorld,
    repo: String,
    step: &cucumber::gherkin::Step,
) -> Result<()> {
    let expected = step
        .docstring
        .as_ref()
        .expect("step requires a docstring")
        .trim()
        .to_string();
    world.run_yx_in_repo(&repo, &["ls", "--format", "pretty"])?;
    let output = world.get_output().trim().to_string();
    if output != expected {
        anyhow::bail!(
            "Expected repo '{}' to have yaks:\n{}\nbut got:\n{}",
            repo,
            expected,
            output
        );
    }
    Ok(())
}

#[then(regex = r#"^([\w-]+) should have these yaks:$"#)]
async fn repo_should_have_yaks_in_process(
    world: &mut InProcessWorld,
    repo: String,
    step: &cucumber::gherkin::Step,
) -> Result<()> {
    let expected = step
        .docstring
        .as_ref()
        .expect("step requires a docstring")
        .trim()
        .to_string();
    world.execute_in_repo(&repo, |app| app.handle(ListYaks::new("pretty", None)))?;
    let output = world.get_repo_output(&repo)?.trim().to_string();
    if output != expected {
        anyhow::bail!(
            "Expected repo '{}' to have yaks:\n{}\nbut got:\n{}",
            repo,
            expected,
            output
        );
    }
    Ok(())
}

#[then(regex = r#"^([\w-]+) should not have a yak called "(.+)"$"#)]
async fn repo_should_not_have_yak(
    world: &mut FullStackWorld,
    repo: String,
    yak: String,
) -> Result<()> {
    world.run_yx_in_repo(&repo, &["ls", "--format", "markdown"])?;
    let output = world.get_output();
    if output.contains(&yak) {
        anyhow::bail!(
            "Expected repo '{}' to NOT have yak '{}', but it was found in output:\n{}",
            repo,
            yak,
            output
        );
    }
    Ok(())
}

#[then(regex = r#"^([\w-]+) should not have a yak called "(.+)"$"#)]
async fn repo_should_not_have_yak_in_process(
    world: &mut InProcessWorld,
    repo: String,
    yak: String,
) -> Result<()> {
    world.execute_in_repo(&repo, |app| app.handle(ListYaks::new("markdown", None)))?;
    let output = world.get_repo_output(&repo)?;
    if output.contains(&yak) {
        anyhow::bail!(
            "Expected repo '{}' to NOT have yak '{}', but it was found in output:\n{}",
            repo,
            yak,
            output
        );
    }
    Ok(())
}

#[then(regex = r#"^([\w-]+) yak "(.+)" should have state "(.+)"$"#)]
async fn repo_yak_should_have_state(
    world: &mut FullStackWorld,
    repo: String,
    yak: String,
    state: String,
) -> Result<()> {
    world.run_yx_in_repo(&repo, &["ls", "--format", "json"])?;
    let output = world.get_output();
    let json: serde_json::Value = serde_json::from_str(&output)
        .context(format!("Failed to parse JSON output: {}", output))?;
    fn find_yak(arr: &[serde_json::Value], name: &str) -> Option<String> {
        for item in arr {
            if item["name"].as_str() == Some(name) {
                return item["state"].as_str().map(|s| s.to_string());
            }
            if let Some(children) = item["children"].as_array() {
                if let Some(state) = find_yak(children, name) {
                    return Some(state);
                }
            }
        }
        None
    }
    let arr = json.as_array().context("Expected JSON array")?;
    let actual_state = find_yak(arr, &yak);
    match actual_state {
        Some(ref s) if s == &state => Ok(()),
        Some(ref s) => anyhow::bail!(
            "Expected yak '{}' in repo '{}' to have state '{}', but it has state '{}'",
            yak,
            repo,
            state,
            s
        ),
        None => anyhow::bail!(
            "Expected yak '{}' in repo '{}', but it was not found in output:\n{}",
            yak,
            repo,
            output
        ),
    }
}

#[then(regex = r#"^([\w-]+) yak "(.+)" should have state "(.+)"$"#)]
async fn repo_yak_should_have_state_in_process(
    world: &mut InProcessWorld,
    repo: String,
    yak: String,
    state: String,
) -> Result<()> {
    world.execute_in_repo(&repo, |app| app.handle(ListYaks::new("json", None)))?;
    let output = world.get_repo_output(&repo)?;
    let json: serde_json::Value = serde_json::from_str(&output)
        .context(format!("Failed to parse JSON output: {}", output))?;
    fn find_yak(arr: &[serde_json::Value], name: &str) -> Option<String> {
        for item in arr {
            if item["name"].as_str() == Some(name) {
                return item["state"].as_str().map(|s| s.to_string());
            }
            if let Some(children) = item["children"].as_array() {
                if let Some(state) = find_yak(children, name) {
                    return Some(state);
                }
            }
        }
        None
    }
    let arr = json.as_array().context("Expected JSON array")?;
    let actual_state = find_yak(arr, &yak);
    match actual_state {
        Some(ref s) if s == &state => Ok(()),
        Some(ref s) => anyhow::bail!(
            "Expected yak '{}' in repo '{}' to have state '{}', but it has state '{}'",
            yak,
            repo,
            state,
            s
        ),
        None => anyhow::bail!(
            "Expected yak '{}' in repo '{}', but it was not found in output:\n{}",
            yak,
            repo,
            output
        ),
    }
}

#[then(regex = r#"^([\w-]+) yak "(.+)" should have context "(.+)"$"#)]
async fn repo_yak_should_have_context(
    world: &mut FullStackWorld,
    repo: String,
    yak: String,
    expected: String,
) -> Result<()> {
    world.run_yx_in_repo(&repo, &["context", "--show", &yak])?;
    let output = world.get_output();
    if !output.contains(&expected) {
        anyhow::bail!(
            "Expected context of yak '{}' in repo '{}' to contain '{}', but got:\n{}",
            yak,
            repo,
            expected,
            output
        );
    }
    Ok(())
}

#[then(regex = r#"^([\w-]+) yak "(.+)" should have context "(.+)"$"#)]
async fn repo_yak_should_have_context_in_process(
    world: &mut InProcessWorld,
    repo: String,
    yak: String,
    expected: String,
) -> Result<()> {
    world.execute_in_repo(&repo, |app| app.handle(ShowContext::new(&yak)))?;
    let output = world.get_repo_output(&repo)?;
    if !output.contains(&expected) {
        anyhow::bail!(
            "Expected context of yak '{}' in repo '{}' to contain '{}', but got:\n{}",
            yak,
            repo,
            expected,
            output
        );
    }
    Ok(())
}

// ============================================================================
// Helper functions
// ============================================================================

fn check_output<W: TestWorld + ?Sized>(world: &W, step: &cucumber::gherkin::Step) -> Result<()> {
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

fn check_yak_count<W: TestWorld + ?Sized>(world: &mut W, expected: usize) -> Result<()> {
    world.list_yaks_with_format("plain")?;
    let output = world.get_output();
    let actual = output.trim().lines().filter(|l| !l.is_empty()).count();
    if actual != expected {
        anyhow::bail!("Expected {} yak(s), but found {}", expected, actual);
    }
    Ok(())
}

fn check_command_fails<W: TestWorld + ?Sized>(world: &W) -> Result<()> {
    if world.get_exit_code() == 0 {
        anyhow::bail!("Expected command to fail, but it succeeded");
    }
    Ok(())
}

fn check_error_contains<W: TestWorld + ?Sized>(world: &W, expected: &str) -> Result<()> {
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

fn check_empty_output<W: TestWorld + ?Sized>(world: &W) -> Result<()> {
    let output = world.get_output();
    let actual = output.trim();
    if !actual.is_empty() {
        anyhow::bail!("\nExpected empty output\n\nActual:\n{}", actual);
    }
    Ok(())
}

fn check_should_succeed<W: TestWorld + ?Sized>(world: &W) -> Result<()> {
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Expected command to succeed, but it failed with exit code {}.\nstderr: {}",
            world.get_exit_code(),
            world.get_error()
        );
    }
    Ok(())
}

fn check_output_includes<W: TestWorld + ?Sized>(world: &W, expected: &str) -> Result<()> {
    let output = world.get_output();
    let output_no_ansi = strip_ansi_codes(&output);
    if !output_no_ansi.contains(expected) {
        anyhow::bail!(
            "Expected output to include '{}', but got:\n{}",
            expected,
            output_no_ansi
        );
    }
    Ok(())
}

fn check_line_of_output_includes<W: TestWorld + ?Sized>(
    world: &W,
    line_num: usize,
    expected: &str,
) -> Result<()> {
    let output = world.get_output();
    let output_no_ansi = strip_ansi_codes(&output);
    let lines: Vec<&str> = output_no_ansi.lines().collect();
    if line_num == 0 || line_num > lines.len() {
        anyhow::bail!(
            "Line {} does not exist. Output has {} line(s):\n{}",
            line_num,
            lines.len(),
            output_no_ansi
        );
    }
    let line = lines[line_num - 1];
    if !line.contains(expected) {
        anyhow::bail!(
            "Expected line {} to include '{}', but got: '{}'",
            line_num,
            expected,
            line
        );
    }
    Ok(())
}

fn check_output_not_includes<W: TestWorld + ?Sized>(world: &W, expected: &str) -> Result<()> {
    let output = world.get_output();
    let output_no_ansi = strip_ansi_codes(&output);
    if output_no_ansi.contains(expected) {
        anyhow::bail!(
            "Expected output to NOT include '{}', but got:\n{}",
            expected,
            output_no_ansi
        );
    }
    Ok(())
}

/// Split a string into arguments, respecting double-quoted strings.
pub fn shell_split(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut has_token = false;
    for c in s.chars() {
        match c {
            '"' => {
                has_token = true;
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if has_token {
                    result.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            _ => {
                has_token = true;
                current.push(c);
            }
        }
    }
    if has_token {
        result.push(current);
    }
    result
}

// ============================================================================
// NO_COLOR and TTY detection steps (full-stack only)
// ============================================================================

#[when(expr = "I list the yaks with NO_COLOR set")]
async fn list_yaks_with_no_color(world: &mut FullStackWorld) -> Result<()> {
    world.run_yx_with_no_color(&["list"])
}

#[when(expr = "I list the yaks piped through cat")]
async fn list_yaks_piped_through_cat(world: &mut FullStackWorld) -> Result<()> {
    // When run via Command::new().output(), stdout is a pipe (not a TTY),
    // so is_terminal() returns false and color is suppressed automatically.
    world.run_raw(&["list"])
}

#[then(expr = "the output should not contain escape codes")]
async fn output_should_not_contain_escape_codes(world: &mut FullStackWorld) -> Result<()> {
    let output = world.get_output();
    if output.contains("\x1b[") {
        anyhow::bail!(
            "Expected no ANSI escape codes in output, but found them:\n{:?}",
            output
        );
    }
    Ok(())
}
