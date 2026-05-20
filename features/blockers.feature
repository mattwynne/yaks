Feature: Explicit blockers affect readiness
  Explicit blocker relationships make otherwise actionable yaks not ready.

  Rule: Explicit yak blockers affect readiness

  Scenario: A yak blocked by another yak is not ready
    Given I add the yak "deploy release"
    And I add the yak "security review"
    When I add blocker "security review" to "deploy release" with reason "waiting on approval"
    And I list ready yaks
    Then the output should not include "deploy release"
    When I list the yaks as json
    Then the JSON yak "deploy release" should have ready false
    And the JSON yak "deploy release" should be blocked by "security review" with reason "waiting on approval"

  Rule: A yak can have at most one manual blocker

  Scenario: Adding another manual blocker replaces the existing manual blocker
    Given I add the yak "publish announcement"
    When I add manual blocker to "publish announcement" with reason "waiting for credentials"
    And I list the yaks as json
    Then the JSON yak "publish announcement" should have ready false
    And the JSON yak "publish announcement" should have exactly one manual blocker with reason "waiting for credentials"
    When I add manual blocker to "publish announcement" with reason "waiting on legal approval"
    And I list the yaks as json
    Then the JSON yak "publish announcement" should have exactly one manual blocker with reason "waiting on legal approval"
    When I remove manual blocker from "publish announcement"
    And I list the yaks as json
    Then the JSON yak "publish announcement" should have ready true
    And the JSON yak "publish announcement" should not have blockers

  Scenario: Manual blockers require a reason
    Given I add the yak "publish announcement"
    When I try to add manual blocker to "publish announcement" without a reason
    Then the command should fail
    And the error should contain "manual blockers require a non-empty --reason"

  Rule: Manual blockers prevent workflow transitions

  Scenario: A yak with an active manual blocker cannot be started or completed
    Given I add the yak "publish announcement"
    And I add manual blocker to "publish announcement" with reason "waiting for credentials"
    When I try to start "publish announcement"
    Then the command should fail
    And the error should contain "cannot start 'publish announcement' - it is not ready"
    And the error should contain "blocked by waiting for credentials"
    When I try to mark the yak "publish announcement" as done
    Then the command should fail
    And the error should contain "cannot mark 'publish announcement' as done - it is blocked by waiting for credentials"
    When I try to set the state of "publish announcement" to "done"
    Then the command should fail
    And the error should contain "cannot mark 'publish announcement' as done - it is blocked by waiting for credentials"
    When I remove manual blocker from "publish announcement"
    And I list the yaks as json
    Then the JSON yak "publish announcement" should have ready true

  Rule: Blocker JSON identifies blocker kinds

  Scenario: JSON distinguishes yak blockers from manual blockers
    Given I add the yak "deploy release"
    And I add the yak "security review"
    When I add blocker "security review" to "deploy release" with reason "waiting on approval"
    And I list the yaks as json
    Then the JSON yak "deploy release" should be blocked by yak "security review"

  Scenario: Completing a blocker makes the blocked yak ready
    Given I add the yak "publish announcement"
    And I add the yak "legal approval"
    And I add blocker "legal approval" to "publish announcement"
    When I mark the yak "legal approval" as done
    And I list the yaks as json
    Then the JSON yak "publish announcement" should have ready true
    And the JSON yak "publish announcement" should not have blockers
    When I list ready yaks
    Then the output should include "publish announcement"
    When I show the log
    Then the output should include "removed blocker"

  Scenario: A yak with an active explicit blocker cannot be marked done
    Given I add the yak "deploy release"
    And I add the yak "security review"
    And I add blocker "security review" to "deploy release"
    When I try to mark the yak "deploy release" as done
    Then the command should fail
    And the error should contain "cannot mark 'deploy release' as done - it is blocked by security review"
    When I list the yaks in "markdown" format
    Then the output should be:
      """
      - [todo] deploy release
      - [todo] security review
      """
    When I show the log
    Then the output should not include "marked deploy release done"

  Scenario: A yak with an active explicit blocker cannot be set to done
    Given I add the yak "deploy release"
    And I add the yak "security review"
    And I add blocker "security review" to "deploy release"
    When I try to set the state of "deploy release" to "done"
    Then the command should fail
    And the error should contain "cannot mark 'deploy release' as done - it is blocked by security review"
    When I list the yaks in "markdown" format
    Then the output should be:
      """
      - [todo] deploy release
      - [todo] security review
      """
    When I show the log
    Then the output should not include "set deploy release state to done"

  Scenario: A blocker can be manually removed
    Given I add the yak "deploy release"
    And I add the yak "security review"
    And I add blocker "security review" to "deploy release"
    When I remove blocker "security review" from "deploy release"
    And I list the yaks as json
    Then the JSON yak "deploy release" should have ready true
    And the JSON yak "deploy release" should not have blockers
    When I list ready yaks
    Then the output should include "deploy release"

  Scenario: Recursively completing a subtree removes blockers supplied by descendants
    Given I add the yak "launch conference"
    And I add the yak "conference prep"
    And I add the yak "book venue" under "conference prep"
    And I add blocker "book venue" to "launch conference"
    When I mark the yak "conference prep" as done recursively
    And I list the yaks as json
    Then the JSON yak "launch conference" should not have blockers
    When I show the log
    Then the output should include "removed blocker"

  Scenario: Removing a blocker yak removes its active explicit blocker relationships
    Given I add the yak "deploy release"
    And I add the yak "security review"
    And I add blocker "security review" to "deploy release"
    When I remove the yak "security review"
    And I list the yaks as json
    Then the JSON yak "deploy release" should not have blockers
    When I show the log
    Then the output should include "removed blocker"

  Scenario: Recursively removing a subtree removes blockers touching descendants
    Given I add the yak "publish announcement"
    And I add the yak "campaign launch"
    And I add the yak "sponsor approval" under "campaign launch"
    And I add blocker "sponsor approval" to "publish announcement"
    When I remove the yak "campaign launch" recursively
    And I list the yaks as json
    Then the JSON yak "publish announcement" should not have blockers
    When I show the log
    Then the output should include "removed blocker"

  Scenario: Pruning a blocker yak removes its active explicit blocker relationships
    Given I add the yak "deploy release"
    And I add the yak "security review"
    And I mark the yak "security review" as done
    And I add blocker "security review" to "deploy release"
    When I prune done yaks
    And I list the yaks as json
    Then the JSON yak "deploy release" should not have blockers
    When I show the log
    Then the output should include "removed blocker"

  Scenario: Updating and clearing blocker reasons
    Given I add the yak "publish announcement"
    And I add the yak "legal approval"
    When I add blocker "legal approval" to "publish announcement" with reason "draft not reviewed"
    And I add blocker "legal approval" to "publish announcement" with reason "approved by legal"
    And I list the yaks as json
    Then the JSON yak "publish announcement" should be blocked by "legal approval" with reason "approved by legal"
    When I add blocker "legal approval" to "publish announcement" with reason ""
    And I list the yaks as json
    Then the JSON yak "publish announcement" should be blocked by "legal approval" without a reason
    When I show the log
    Then the output should include "updated blocker"

  Rule: Blocker commands are idempotent and helpful

    Example: Adding the same explicit blocker again fails with nothing changed
      Given I add the yak "deploy release"
      And I add the yak "security review"
      And I add blocker "security review" to "deploy release"
      When I try to add blocker "security review" to "deploy release"
      Then the command should fail
      And the error should contain "already blocks"
      And the error should contain "nothing changed"
      When I show the log
      Then the output should not include "updated blocker"

    Example: Adding the same explicit blocker with no reason preserves an existing reason
      Given I add the yak "publish announcement"
      And I add the yak "legal approval"
      And I add blocker "legal approval" to "publish announcement" with reason "awaiting sign-off"
      When I try to add blocker "legal approval" to "publish announcement"
      Then the command should fail
      And the error should contain "already blocks"
      And the error should contain "nothing changed"
      When I list the yaks as json
      Then the JSON yak "publish announcement" should be blocked by "legal approval" with reason "awaiting sign-off"
      When I show the log
      Then the output should not include "updated blocker"

    Example: Adding the same explicit blocker reason again fails with nothing changed
      Given I add the yak "publish announcement"
      And I add the yak "legal approval"
      And I add blocker "legal approval" to "publish announcement" with reason "awaiting sign-off"
      When I try to add blocker "legal approval" to "publish announcement" with reason "awaiting sign-off"
      Then the command should fail
      And the error should contain "already blocks"
      And the error should contain "nothing changed"
      When I show the log
      Then the output should not include "updated blocker"

    Example: Updating an explicit blocker reason emits an update
      Given I add the yak "publish announcement"
      And I add the yak "legal approval"
      And I add blocker "legal approval" to "publish announcement" with reason "draft not reviewed"
      When I add blocker "legal approval" to "publish announcement" with reason "approved by legal"
      Then the output should include "Updated blocker"
      When I show the log
      Then the output should include "updated blocker"

    Example: Removing an absent explicit blocker advises nothing changed
      Given I add the yak "deploy release"
      And I add the yak "security review"
      When I remove blocker "security review" from "deploy release"
      Then the output should include "No active explicit blocker"
      And the output should include "nothing changed"
      When I show the log
      Then the output should not include "removed blocker"

    Example: Adding a blocker already provided by hierarchy advises no explicit blocker was added
      Given I add the yak "launch conference"
      And I add the yak "book venue" under "launch conference"
      When I add blocker "book venue" to "launch conference"
      Then the output should include "already blocks"
      And the output should include "no explicit blocker added"
      When I show the log
      Then the output should not include "marked launch conference blocked by book venue"

  Rule: Blocker commands reject cycles

    Example: A yak cannot block itself
      Given I add the yak "yak"
      When I try to add blocker "yak" to "yak"
      Then the command should fail
      And the error should contain "cannot block itself"
      When I show the log
      Then the output should not include "marked yak blocked by yak"

    Example: Mutual explicit blockers are rejected
      Given I add the yak "a"
      And I add the yak "b"
      And I add blocker "b" to "a"
      When I try to add blocker "a" to "b"
      Then the command should fail
      And the error should contain "would create circular dependency"
      When I show the log
      Then the output should not include "marked b blocked by a"

    Example: Longer explicit circular dependencies are rejected
      Given I add the yak "a"
      And I add the yak "b"
      And I add the yak "c"
      And I add blocker "b" to "a"
      And I add blocker "c" to "b"
      When I try to add blocker "a" to "c"
      Then the command should fail
      And the error should contain "would create circular dependency"
      When I show the log
      Then the output should not include "marked c blocked by a"

    Example: An ancestor cannot explicitly block a descendant
      Given I add the yak "parent"
      And I add the yak "child" under "parent"
      When I try to add blocker "parent" to "child"
      Then the command should fail
      And the error should contain "would create circular dependency"
      When I show the log
      Then the output should not include "marked child blocked by parent"

    Example: A multi-ancestor hierarchy cannot explicitly block a descendant
      Given I add the yak "parent"
      And I add the yak "child" under "parent"
      And I add the yak "grandchild" under "child"
      When I try to add blocker "parent" to "grandchild"
      Then the command should fail
      And the error should contain "would create circular dependency"
      When I show the log
      Then the output should not include "marked grandchild blocked by parent"
