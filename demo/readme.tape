Output output/README.gif

Require git
Require shelfbox

Set Shell "bash"
Set FontFamily "DejaVu Sans Mono"
Set FontSize 22
Set Width 1000
Set Height 600
Set Padding 20
Set Margin 24
Set MarginFill "#11111b"
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms
Set CursorBlink false
Set WindowBar Colorful

Env PS1 "$ "

# Keep the generated Git repository and its setup out of the recording.
Hide
Type "./setup.sh --reset && cd workspace/sample-repo && clear"
Sleep 400ms
Enter
Sleep 1000ms
Show

Type "pwd"
Sleep 400ms
Enter
Sleep 2200ms

Type "ls -l .env"
Sleep 400ms
Enter
Sleep 2200ms

Type "git status --short"
Sleep 400ms
Enter
Sleep 2200ms

Type "shelfbox item add .env"
Sleep 400ms
Enter
Sleep 2200ms

Type "ls -l .env"
Sleep 400ms
Enter
Sleep 2200ms

Type "git status --short"
Sleep 400ms
Enter
Sleep 2200ms

Type "shelfbox repo list --format plain"
Sleep 400ms
Enter
Sleep 2200ms

Type "shelfbox item list --format plain"
Sleep 400ms
Enter
Sleep 5000ms
