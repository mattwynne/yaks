@fullstack
Feature: Agent-aware help examples
  The help output shows different examples depending on whether
  yx is running inside an AI agent session or a human terminal.

  Rule: Humans see human-friendly examples

    Example: Default help shows human examples
      When I request help outside an agent session
      Then the output should include "yx start"
      And the output should not include "--format json"

    Example: Bare yx shows human examples on stderr
      When I run bare yx outside an agent session
      Then the error should contain "yx start"

  Rule: Agents see agent-friendly examples

    Example: Help in agent session shows agent examples
      When I request help in an agent session
      Then the output should include "--format json"
      And the output should not include "yx start"

    Example: Bare yx in agent session shows agent examples on stderr
      When I run bare yx in an agent session
      Then the error should contain "--format json"
