Feature: Show yak details

  Rule: Done children are shown before incomplete children

    @fullstack
    Example: Children in mixed states are sorted done-first, then alphabetically
      Given I have a clean git repository
      And I add the yak "project"
      And I add the yak "alpha" under "project"
      And I add the yak "zeta" under "project"
      And I mark the yak "zeta" as done
      When I run yx show "project"
      Then "zeta" appears before "alpha" in the output
