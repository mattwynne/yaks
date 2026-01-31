## Fix Gitignore Check Test

**The Problem:**
The check_gitignore() function works perfectly when tested manually, but fails in shellspec test environment. The test expects yx to fail when .yaks is not in .gitignore, but it succeeds instead.

**Why This Happens:**
Shellspec's `When run` executes in a subshell, so the `cd "$temp_dir"` before it doesn't affect where yx runs. This means GIT_WORK_TREE resolves to the wrong directory.

**What We Tried:**
1. ✅ Using `git -c core.excludesfile=/dev/null` to bypass global gitignore
2. ✅ Modified test to use `sh -c "cd '$temp_dir' && yx ls"` pattern
3. ❌ Test still fails - shellspec may be caching or there's another issue

**Manual Test (Works!):**
```bash
cd /tmp/test && git init && yx ls
# Correctly fails with: Error: .yaks folder is not gitignored
```

**Test File:**
`spec/features/gitignore_check.sh`

**Implementation:**
`bin/yx:12-18` - check_gitignore() function

**What Needs Investigation:**
- Why does shellspec test still fail after fixing the cd issue?
- Is there shellspec caching involved?
- Should we set GIT_WORK_TREE explicitly in the test?
- Consider looking at how git_repo_check.sh test works (it uses same pattern)

**Done Looks Like:**
`shellspec spec/features/gitignore_check.sh` passes (exits with failure when .gitignore missing)
