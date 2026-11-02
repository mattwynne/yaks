@fullstack
Feature: Agent-aware help examples
  The help output shows different examples depending on whether
  yx is running inside an AI agent session or a human terminal.

  Rule: Humans see human-friendly examples

    Example: Human help distinguishes nested prerequisites from explicit blockers
      When I request help outside an agent session
      Then the output should include "Use the tree for prerequisites, not just categories"
      And the output should include "Use explicit yak blockers for shared/cross-branch dependencies"

    Example: Default help shows human examples
      When I request help outside an agent session
      Then the output should include "yx start"
      And the output should not include "--format json"

    Example: Bare yx shows human examples on stderr
      When I run bare yx outside an agent session
      Then the error should contain "yx start"

  Rule: Agents see agent-friendly examples

    Example: Agent help distinguishes nested prerequisites from explicit blockers
      When I request help in an agent session
      Then the output should include "Use the tree for prerequisites, not just categories"
      And the output should include "Use explicit yak blockers for shared/cross-branch dependencies"

    Example: Help in agent session shows agent examples
      When I request help in an agent session
      Then the output should include "--format json"
      And the output should not include "yx start"

    Example: Bare yx in agent session shows agent examples on stderr
      When I run bare yx in an agent session
      Then the error should contain "--format json"

  Rule: Command help explains how to model dependencies

    Example: Add help explains prerequisite nesting direction
      When I run yx add --help
      Then it should succeed
      And the output should include "If B must happen before A, put B under A"
      And the output should include "Earlier work is the child; later work is the parent"

    Example: Move help shows how to replace a flat blocker chain with nesting
      When I run yx move --help
      Then it should succeed
      And the output should include "Avoid a flat list of sibling yaks joined by explicit blockers"
      And the output should include "yx move Deploy service --under Release inventory tools"
      And the output should include "yx move Integrate adapters --under Deploy service"

    Example: Blocker add help reserves explicit blockers for non-tree dependencies
      When I run yx blocker add --help
      Then it should succeed
      And the output should include "shared/cross-branch dependency"
      And the output should include "Do not duplicate a dependency already implied by nesting"
      And the output should include "A blocker on a parent does not gate its children"
      And the output should include "block each affected leaf"
