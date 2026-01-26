I want to not have to type out the entire path when referring to a yak. I want the yak to be found by a minimal string, or error if it's
not unique enough.


e.g.

yx add ideas/buy a pony
yx add ideas/fix the build
yx add ideas/fix the fridge
yx done build # => marks "ideas/fix the build" as done
yx done fix   # => fails with "did you mean..." prompt
