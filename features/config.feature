Feature: User configuration
  Manage user-level configuration settings for yx.

  Config is stored in $XDG_CONFIG_HOME/yaks/config.toml (defaults
  to ~/.config/yaks/config.toml). Settings persist across sessions
  and are not tied to any particular yak repository.

  Rule: Unknown config keys are rejected

    Example: Get rejects unknown key
      Given I have a clean git repository
      When I get config "no-such-key"
      Then the command should fail
      And the error should contain "Unknown config key"

    Example: Set rejects unknown key
      Given I have a clean git repository
      When I set config "no-such-key" to "value"
      Then the command should fail
      And the error should contain "Unknown config key"

  Rule: Config values can be listed

    Example: List shows nothing when no keys are defined
      Given I have a clean git repository
      When I list config
      Then the output should be empty
