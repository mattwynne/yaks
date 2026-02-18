Feature: Tab completion
  The yx CLI supports tab completion for commands, arguments, and flags.
  Completion suggestions are context-aware, filtering based on yak state
  and the current command being typed.

  Rule: Command name completion
    Completions for the top-level command suggest all available subcommands.

    Example: Completing after "yx" suggests available commands
      Given I have a clean git repository
      When I run yx completions -- yx ""
      Then it should succeed
      And the output should include "add"
      And the output should include "done"
      And the output should include "remove"

  Rule: Argument completion filters by context
    When completing arguments for a command, only contextually valid
    values are suggested. For example, the "done" command should not
    suggest yaks that are already done.

    Example: Completing "done" only suggests yaks that are not already done
      Given I have a clean git repository
      And I add the yak "todo-yak"
      And I add the yak "done-yak"
      And I mark the yak "done-yak" as done
      When I run yx completions -- yx done ""
      Then the output should include "todo-yak"
      And the output should not include "done-yak"

  Rule: Bash completion wiring
    The bash completion script integrates with bash's programmable
    completion system, handling flag suggestions and yak names.

    @bash_completion
    Example: Completing add suggests existing yak names
      Given I have a clean git repository
      And I add the yak "grandma"
      And I add the yak "mummy" under "grandma"
      When I invoke bash completion for words: yx add ""
      Then the completions should include "grandma"
      And the completions should include "mummy"

    @bash_completion
    Example: Completing after "--" suggests flags for the command
      Given I have a clean git repository
      When I invoke bash completion for words: yx done "--"
      Then the completions should include "--recursive"
