## Domain validation logic

import types
import std/strutils

const ForbiddenChars = ['\\', ':', '*', '?', '|', '<', '>', '"']

proc validateYakName*(name: string) =
  ## Validates a yak name according to business rules
  for ch in name:
    if ch in ForbiddenChars:
      raise newException(InvalidNameError,
        "Invalid yak name: contains forbidden characters (\\ : * ? | < > \")")
