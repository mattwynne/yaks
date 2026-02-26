# VHS demo tape for README
# Run with: dev demo

Output demo.gif

Set Shell zsh
Set FontSize 24
Set Width 900
Set Height 500
Set Padding 20
Set TypingSpeed 50ms

# Alice creates some yaks
Type "cd alice"
Enter
Sleep 500ms

Type "yx add 'make the tea'"
Enter
Sleep 500ms

Type "yx add 'boil the kettle' --under tea"
Enter
Sleep 500ms

Type "yx add 'find a mug' --under tea"
Enter
Sleep 500ms

Type "yx add 'wash a mug' --under mug"
Enter
Sleep 500ms

Type "yx ls"
Enter
Sleep 2s

# Alice starts working and completes some yaks
Type "yx start wash"
Enter
Sleep 500ms

Type "yx ls"
Enter
Sleep 2s

Type "yx done 'find a mug' --recursive"
Enter
Sleep 500ms

Type "yx ls"
Enter
Sleep 2s

# Alice syncs her work
Type "yx sync"
Enter
Sleep 2s

Hide
Type "cd .. && clear"
Enter
Show

# Bob syncs and sees Alice's yaks
Type "cd bob"
Enter
Sleep 500ms

Type "yx sync"
Enter
Sleep 2s

Type "yx ls"
Enter
Sleep 2s

# Bob adds a yak and completes one
Type "yx add 'buy biscuits' --under tea"
Enter
Sleep 500ms

Type "yx done 'boil the kettle'"
Enter
Sleep 500ms

Type "yx ls"
Enter
Sleep 2s

# Bob syncs back
Type "yx sync"
Enter
Sleep 2s

Hide
Type "cd .. && clear"
Enter
Show

# Alice pulls Bob's changes
Type "cd alice"
Enter
Sleep 500ms

Type "yx sync"
Enter
Sleep 2s

Type "yx ls"
Enter
Sleep 5s
