The way git works, each sub-command is a separate executable.

These executables are layers, with "plumbing" commands at the lower level and "porcelain" commands that wrap the lower level commands to do useful tasks for the user.

I'd like to reorganize our commands in this way. Now that we have a release script and a way of testing it, we should be safer to bundle multiple files into a release.
