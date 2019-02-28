# Feature

- Can move to the project folder
- git command helper
- fetch, merge, status for multiple project
- CUI for git add/reset, commit
- CUI for git push from commit message
- CUI for find
- CUI for grep

# Advantage

- The convenient CUI interface based on urwid(CUI)
- Can manage multiple git repositories conveniently
- Can do commit with seeing modification for commit msg under CUI
- Can push conveniently with commit message

# How to install

## Environment setup

\*\* if you have installed pip3 and virtualenv already, skip it.

\$ sudo apt install python3 python3-pip git
\$ sudo pip3 install virtualenv

## Install
\$ curl -L https://github.com/inertry/synapbookCommander/raw/stable/install.sh | bash -

or if you clone the repository already,
$ echo ". $(pwd)/bash-script.sh" >> ~/.bashrc

# How to use

Type 'dc'

## main ui

- `j/k` - move focus
- `u` - move upper folder
- `h/enter` - enter the folder
- `E` - edit the file

- `n` - Mark current item as trivial
- `m` - Mark current item as important

- `f` - filtering current folder items
- `s` - running shell command with \$ for current selected file name

- `/` - command mode
- reg - Register current folder
- find/ff - Running find command with CUI result
  ex> ff \*\*.py
- grep/gg - Running grep command with CUI result
  ex> gg Metric

- `R` - Register/unregister this selected folder
- `L` - Show the list of repository folders
- `C` - Show commit dialog for current repo

### workspace

- `Alt+Right/Left` - Add or remove the folder in Workspace
- `Alt+Up/Dowm` - Move workspace between folder list

## Commit helper(C key)

- `A` - git add current file
- `P` - prompty for git add -p command
- `R` - reset current staged file
- `D` - drop modification
- `[`/`]` - move next/previous file
- `J`/`K` - scroll down/up - you can use arrow key too
- `C` - popup commit dialog
- `F4`/`Q` - quit the program

### on commit dialog,

\*\* you can see all staged modification for input commit message

- up/down - scroll current file's content
- f9/f10 - prev/next file to see modification

\*\* then input commit message and then enter to do commit

## Register folder list(L key)

- `P` - pull --rebase all repo
- `Enter` - move to the selected repo

# Commandline commends

\$ dcf -name "\*\*.py"

\$ dcg "printf"

## update all project repositories

\$ dc update

\*\* all projects that regsitered as repo are updated(git fetch + rebase to remote tracking branch + print status)

\*\* `update` command is comprised of `dc fetch` + `dc merge` + `dc st(status)`

## print all project's status

\$ dc st

\*\* you can combinate several command as follows,

\$ dc fetch st

\*\* you can print current folder or certain repo's status as well

\$ dc update .

\$ dc update plus

## git CUI helper for add/reset, commit

\$ dc ci

\*\* you can use the following keys

## git push

\*\* you can conveniently push commits just specify target branch name

\*\* push command always check tracking branch firstly than ask you to rebase onto.

\$ dc push

\*\* just type target branch you want to push to
