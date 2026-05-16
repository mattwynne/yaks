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
    When I add blocker "blocking yak" to "blocked yak"
    And I show the log
    Then the output should include "updated blocker"
