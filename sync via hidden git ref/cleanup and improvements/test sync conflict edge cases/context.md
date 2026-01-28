Add tests for sync conflict edge cases that aren't currently covered.

After studying the sync implementation (lines 519-565 in bin/yx), several
edge cases emerged that should be tested:

## 1. Delete vs Modify Conflicts
- User1 removes a yak
- User2 modifies it (marks done or adds context)
- Classic git conflict: which version wins?
- Current behavior (line 525): "local files win" - but is delete handled?

## 2. Opposite State Transitions
- User1 marks yak done
- User2 marks same yak undone (done --undo) 
- Racing state changes in opposite directions
- Test which state wins and why

## 3. Context Merge Conflicts
- User1 adds context "A" to yak
- User2 adds context "B" to same yak
- Currently: last-write-wins (line 525 cp overwrites)
- Should verify one context completely replaces the other (no merge)

## 4. Nested Deletion
- User1 removes parent yak (which has children)
- User2 adds more children to that parent
- Does parent come back? Do orphaned children remain?
- Complex directory structure interaction

## 5. Prune Conflicts
- User1 prunes all done yaks
- User2 adds context to a yak that User1 is pruning
- Deletion during modification
- Similar to #1 but via prune command

## 6. Three-way Edit (Both Users Modify Different Aspects)
- Start: yak exists with state=todo, no context
- User1 adds context
- User2 marks done
- Both sync - should get done yak with context (merge both changes)
- This should work with git merge-tree

Most critical: #1 (delete vs modify) - classic distributed conflict scenario.
