Feature: Explicit blockers affect readiness
  Explicit blocker relationships make otherwise actionable yaks not ready.

  Scenario: A yak blocked by another yak is not ready
    Given I add the yak "blocked yak"
    And I add the yak "blocking yak"
    When I add blocker "blocking yak" to "blocked yak" with reason "waiting on it"
    And I list ready yaks
    Then the output should not include "blocked yak"
    When I list the yaks as json
    Then the JSON yak "blocked yak" should have ready false
    And the JSON yak "blocked yak" should be blocked by "blocking yak" with reason "waiting on it"

  Scenario: Completing a blocker makes the blocked yak ready
    Given I add the yak "blocked yak"
    And I add the yak "blocking yak"
    And I add blocker "blocking yak" to "blocked yak"
    When I mark the yak "blocking yak" as done
    And I list the yaks as json
    Then the JSON yak "blocked yak" should have ready true
    And the JSON yak "blocked yak" should not have blockers
    When I list ready yaks
    Then the output should include "blocked yak"
    When I show the log
    Then the output should include "removed blocker"

  Scenario: A blocker can be manually removed
    Given I add the yak "blocked yak"
    And I add the yak "blocking yak"
    And I add blocker "blocking yak" to "blocked yak"
    When I remove blocker "blocking yak" from "blocked yak"
    And I list the yaks as json
    Then the JSON yak "blocked yak" should have ready true
    And the JSON yak "blocked yak" should not have blockers
    When I list ready yaks
    Then the output should include "blocked yak"

  Scenario: Updating and clearing blocker reasons
    Given I add the yak "blocked yak"
    And I add the yak "blocking yak"
    When I add blocker "blocking yak" to "blocked yak" with reason "first reason"
    And I add blocker "blocking yak" to "blocked yak" with reason "second reason"
    And I list the yaks as json
    Then the JSON yak "blocked yak" should be blocked by "blocking yak" with reason "second reason"
    When I add blocker "blocking yak" to "blocked yak" with reason ""
    And I list the yaks as json
    Then the JSON yak "blocked yak" should be blocked by "blocking yak" without a reason
    When I show the log
    Then the output should include "updated blocker"

  Rule: Blocker commands are idempotent and helpful

    Example: Adding the same explicit blocker again fails with nothing changed
      Given I add the yak "blocked yak"
      And I add the yak "blocking yak"
      And I add blocker "blocking yak" to "blocked yak"
      When I try to add blocker "blocking yak" to "blocked yak"
      Then the command should fail
      And the error should contain "already blocks"
      And the error should contain "nothing changed"
      When I show the log
      Then the output should not include "updated blocker"

    Example: Adding the same explicit blocker with no reason preserves an existing reason
      Given I add the yak "blocked yak"
      And I add the yak "blocking yak"
      And I add blocker "blocking yak" to "blocked yak" with reason "waiting"
      When I try to add blocker "blocking yak" to "blocked yak"
      Then the command should fail
      And the error should contain "already blocks"
      And the error should contain "nothing changed"
      When I list the yaks as json
      Then the JSON yak "blocked yak" should be blocked by "blocking yak" with reason "waiting"
      When I show the log
      Then the output should not include "updated blocker"

    Example: Adding the same explicit blocker reason again fails with nothing changed
      Given I add the yak "blocked yak"
      And I add the yak "blocking yak"
      And I add blocker "blocking yak" to "blocked yak" with reason "waiting"
      When I try to add blocker "blocking yak" to "blocked yak" with reason "waiting"
      Then the command should fail
      And the error should contain "already blocks"
      And the error should contain "nothing changed"
      When I show the log
      Then the output should not include "updated blocker"

    Example: Updating an explicit blocker reason emits an update
      Given I add the yak "blocked yak"
      And I add the yak "blocking yak"
      And I add blocker "blocking yak" to "blocked yak" with reason "old reason"
      When I add blocker "blocking yak" to "blocked yak" with reason "new reason"
      Then the output should include "Updated blocker"
      When I show the log
      Then the output should include "updated blocker"

    Example: Removing an absent explicit blocker advises nothing changed
      Given I add the yak "blocked yak"
      And I add the yak "blocking yak"
      When I remove blocker "blocking yak" from "blocked yak"
      Then the output should include "No active explicit blocker"
      And the output should include "nothing changed"
      When I show the log
      Then the output should not include "removed blocker"

    Example: Adding a blocker already provided by hierarchy advises no explicit blocker was added
      Given I add the yak "parent"
      And I add the yak "child" under "parent"
      When I add blocker "child" to "parent"
      Then the output should include "already blocks"
      And the output should include "through hierarchy"
      And the output should include "no explicit blocker added"
      When I show the log
      Then the output should not include "marked parent blocked by child"
