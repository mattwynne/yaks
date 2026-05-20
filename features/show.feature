Feature: Show yak details

  @fullstack
  Rule: Readiness is explained

    Example: Ready todo leaf yak shows it is ready
      Given I have a clean git repository
      And I add the yak "deploy release"
      When I run yx show "deploy release"
      Then the output should include "Ready: yes"

    Example: Todo parent shows incomplete children as the readiness reason
      Given I have a clean git repository
      And I add the yak "deploy release"
      And I add the yak "security review" under "deploy release"
      When I run yx show "deploy release"
      Then the output should include "Ready: no"
      And the output should include "has incomplete children: deploy release/security review"

    Example: Yak blocker is shown as the readiness reason in show and tree
      Given I have a clean git repository
      And I add the yak "deploy release"
      And I add the yak "security review"
      And I add blocker "security review" to "deploy release" with reason "waiting on approval"
      When I run yx show "deploy release"
      Then the output should include "blocked by security review: waiting on approval"
      When I list the yaks
      Then the output should include "not ready: blocked by security review: waiting on approval"

    Example: Manual blocker is shown as the readiness reason in show and tree
      Given I have a clean git repository
      And I add the yak "publish announcement"
      And I add manual blocker to "publish announcement" with reason "waiting for credentials"
      When I run yx show "publish announcement"
      Then the output should include "blocked by manual reason: waiting for credentials"
      When I list the yaks
      Then the output should include "not ready: blocked by manual reason: waiting for credentials"

    Example: Wip yak shows state as the readiness reason
      Given I have a clean git repository
      And I add the yak "deploy release"
      When I start "deploy release"
      And I run yx show "deploy release"
      Then the output should include "Ready: no"
      And the output should include "state is wip"

    Example: Done yak shows state as the readiness reason
      Given I have a clean git repository
      And I add the yak "deploy release"
      And I mark the yak "deploy release" as done
      When I run yx show "deploy release"
      Then the output should include "Ready: no"
      And the output should include "state is done"

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
