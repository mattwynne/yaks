When moving a yak, allow specifying just the parent name as destination:

  yx move foo parent

Should be equivalent to:

  yx move foo parent/foo

This makes it easier to organize yaks without having to type the full path when you just want to move something under a parent while keeping its name.
