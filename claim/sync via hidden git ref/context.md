This one is going to be tricky, but key.

We need to be able to pass the yak map around between clones and worktrees of the repo.

My though is to use a hidden git ref, a bit like an orphaned branch, to contain the yak map files. We can then use git to push/pull/merge that. hopefully our directory structure should minimize merge conflict.

the tests should set up scenarios with three git repos: two "users" who have cloned an origin. it should make sure that they can push and pull their yak maps and sync up. ideally we'll do the sync as soon as we change anything, automatically.
