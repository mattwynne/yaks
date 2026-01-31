# Fix Failing Tests

Two tests are currently failing (106/108 passing):

## 1. gitignore check test (spec/features/gitignore_check.sh:3)
Test: "shows error when .yaks is not gitignored"
- This test is checking that yx warns when `.yaks` is not in .gitignore

## 2. install.sh test (spec/features/install.sh:3)
Test: "installs yx from release zip and runs smoke tests"
- Error: `cp: cannot stat '/Users/mattwynne/git/mattwynne/yaks/result/yx.zip': No such file or directory`
- The test expects a Nix build artifact at `result/yx.zip` which is missing
- This appears to be a build/dependency issue

## Acceptance Criteria
- All 108 tests pass when running `shellspec`
- Both tests run successfully without errors
