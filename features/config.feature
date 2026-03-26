Feature: User configuration
  Manage user-level configuration settings for yx.

  Config is stored in $XDG_CONFIG_HOME/yaks/config.toml (defaults
  to ~/.config/yaks/config.toml). Settings persist across sessions
  and are not tied to any particular yak repository.

  Rule: Config values can be set and retrieved

    Example: Set and get a config value
      Given I have a clean git repository
      When I set config "show-claude-plugin-hint" to "false"
      And I get config "show-claude-plugin-hint"
      Then the output should be:
        """
        false
        """

    Example: Get returns default when key is unset
      Given I have a clean git repository
      When I get config "show-claude-plugin-hint"
      Then the output should be:
        """
        true
        """

  Rule: Config values can be listed

    Example: List shows all config with current values
      Given I have a clean git repository
      When I set config "show-claude-plugin-hint" to "false"
      And I list config
      Then the output should include "show-claude-plugin-hint = false"

    Example: List shows defaults when nothing is set
      Given I have a clean git repository
      When I list config
      Then the output should include "show-claude-plugin-hint = true"
