@fullstack
Feature: Claude plugin install hint
  When running inside Claude Code, nudge the user to install the
  yx plugin if it's not already installed.

  The banner appears on stderr (so it doesn't interfere with
  stdout output) only on bare `yx` and `yx --help`.

  Rule: Banner shows when in Claude Code and plugin not installed

    Example: Banner on bare yx in Claude Code session
      Given I have a clean git repository
      And the Claude Code plugin is not installed
      When I run bare yx in a Claude Code session
      Then stderr should contain the plugin install hint

    Example: Banner on yx --help in Claude Code session
      Given I have a clean git repository
      And the Claude Code plugin is not installed
      When I request help in a Claude Code session
      Then stderr should contain the plugin install hint

  Rule: Banner does not show when plugin is installed

    Example: No banner when plugin is already installed
      Given I have a clean git repository
      And the Claude Code plugin is installed
      When I run bare yx in a Claude Code session
      Then stderr should not contain the plugin install hint

  Rule: Banner does not show when suppressed by config

    Example: No banner when config suppresses it
      Given I have a clean git repository
      And the Claude Code plugin is not installed
      When I set config "show-claude-plugin-hint" to "false"
      And I run bare yx in a Claude Code session
      Then stderr should not contain the plugin install hint

  Rule: Banner does not show outside Claude Code sessions

    Example: No banner when not in Claude Code
      Given I have a clean git repository
      And the Claude Code plugin is not installed
      When I run bare yx outside a Claude Code session
      Then stderr should not contain the plugin install hint
