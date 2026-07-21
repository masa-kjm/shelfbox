Output output/README.gif

Require git
Require shelfbox

Set Shell "bash"
Set FontFamily "DejaVu Sans Mono"
Set FontSize 22
Set Width 900
Set Height 540
Set Padding 20
Set Margin 12
Set MarginFill "#1e1e2e"
Set Theme "Catppuccin Frappe"
Set TypingSpeed 25ms
Set CursorBlink false

Env PS1 "demo $ "

# Keep the generated Git repository and its setup out of the recording.
Hide
Type "./setup.sh --reset && cd workspace/repo && clear"
Enter
Sleep 1s
Show

Type "shelfbox --store ../store item add .env"
Enter
Sleep 2s

Type "shelfbox --store ../store item list"
Enter
Sleep 2s

Type "shelfbox --store ../store item status"
Enter
Sleep 3s
