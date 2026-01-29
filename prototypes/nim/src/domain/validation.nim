## Domain validation logic
##
## Contains business rules for validating yak names and other domain constraints.

import types

const ForbiddenChars = ['\\', ':', '*', '?', '|', '<', '>', '"']
  ## Characters that cannot be used in yak names (filesystem restrictions)

proc validateYakName*(name: string) =
  ## Validates a yak name according to business rules.
  ##
  ## Ensures the name doesn't contain characters that would cause issues
  ## with filesystem storage or path parsing.
  ##
  ## Raises:
  ##   InvalidNameError: If the name contains forbidden characters
  for ch in name:
    if ch in ForbiddenChars:
      raise newException(InvalidNameError,
        "Invalid yak name: contains forbidden characters (\\ : * ? | < > \")")
