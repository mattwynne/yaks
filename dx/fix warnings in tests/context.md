# Fix Test Warnings

Found 2 warnings when running shellspec:

## 1. spec/sync.sh:35 - "pushes yaks to origin"
- Git command: `git show-ref refs/notes/yaks`
- Issue: Produces stdout but test has no expectation for it
- Output: `ffa32734ed3639de764420ec1371034a78ac2c9d refs/notes/yaks`

## 2. spec/sync_unit.sh:21 - "creates refs/notes/yaks when yak exists"  
- Git command: `git rev-parse refs/notes/yaks`
- Issue: Produces stdout but test has no expectation for it
- Output: `3c224c78f5597ad440cf4848803c1380b7010ba5`

## Solution
Add stdout expectations to these tests to handle the git command output.
