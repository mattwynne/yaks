Now that we have a proper release script, I think we can start shipping argc in our release and depending on it from the bash script, which will make argument parsing a lot simpler.

We are done when the bin/yx script uses argc for command-line options parsing, and all the tests pass including the installer test.
