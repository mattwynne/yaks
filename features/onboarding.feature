Feature: First-time onboarding
  When someone runs yx for the first time in a git repo,
  we welcome them and help set things up.

  Rule: Welcome new users and set up .gitignore

    @fullstack
    Example: First-time user accepts full setup
      Given a git repository that has never used yaks
      When I interactively run yx add "my-yak" from this directory
      And I accept the offer to add .yaks to .gitignore
      And I accept the offer to commit the change
      Then the command should succeed
      And there should be a yak called "my-yak"
      And .yaks should be in .gitignore
      And the last commit should include .gitignore

    @fullstack
    Example: First-time user accepts .gitignore but skips commit
      Given a git repository that has never used yaks
      When I interactively run yx add "my-yak" from this directory
      And I accept the offer to add .yaks to .gitignore
      And I decline the offer to commit the change
      Then the command should succeed
      And there should be a yak called "my-yak"
      And .yaks should be in .gitignore
      And .gitignore should have uncommitted changes
      And the output should contain "remember to commit"

    @fullstack
    Example: First-time user declines setup
      Given a git repository that has never used yaks
      When I interactively run yx add "my-yak" from this directory
      And I decline the offer to add .yaks to .gitignore
      Then the command should fail
      And .yaks should not be in .gitignore

    @fullstack
    Example: Non-interactive first-time use fails with helpful message
      Given a git repository that has never used yaks
      When I non-interactively run yx from this directory
      Then the command should fail
      And the error should contain ".yaks is not gitignored. Fix with: echo '.yaks' >> .gitignore"
