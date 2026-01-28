Yaks has its own way of managing state in the .yaks folder. It's essential this folder is not checked in to git by the user.

We should run a check to see whether the .yaks folder is gitignored, and if not we should fail.

As a second iteration, we can offer to add git configuration to ignore it.
