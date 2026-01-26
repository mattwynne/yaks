The `yx ls` command is displaying the nesting hierarchy incorrectly.

**What's happening:**
The output shows:
```
- [ ] fix list bugs
  - [ ] fix problem with sort when a child is done
  - [ ] sync via hidden git ref
  - [ ] incorrect nesting display in ls output
    - [ ] add lockfile to prevent concurrent syncs
```

**What it should show:**
```
- [ ] claim
  - [ ] sync via hidden git ref
    - [ ] add lockfile to prevent concurrent syncs
- [ ] fix list bugs
  - [ ] fix problem with sort when a child is done
  - [ ] incorrect nesting display in ls output
```

**The bug:**
Yaks from different parent directories are being grouped/nested incorrectly. The display logic in `list_yaks()` (bin/yx:26+) is not properly tracking the full path hierarchy when displaying nested yaks.

**Root cause likely in:**
- The sorting logic (line 36-47) that processes depth and mtime
- The display logic (lines 60-78) that determines indentation
- How parent-child relationships are inferred from paths
