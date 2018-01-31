
# Feature
* change project folder
* git command helper 
 * fetch, merge, status for multiple project
 * CUI for git add/reset, commit
 * CUI for git push from commit message
* CUI for find(Alpha)
* CUI for ack-grep(Alpha)

# Advantage
* The convenient CUI interface based on urwid(CUI)
* Can manage multiple git repositories conveniently
* Can do commit with seeing modification for commit msg under CUI
* Can push conveniently with commit message


# How to install

## Environment setup

$ apt-get install python3-setuptools

$ easy_install-3.4 pip

$ pip3 install click urwid


## Install 

$ mkdir ~/tool

$ git clone https://github.com/cjng96/devCmdTool.git

$ vi ~/.bashrc
** append the following lines
DEV_CMD_PATH=~/tool/devCmdTool

. $DEV_CMD_PATH/bash-script.sh

$ dc config

$ vi path.py

** write your path.py file as follows
```python
gp=~/git
pathList = [
  dict(name="coll", path=os.path.join(gp, "collector"), repo=1),
  dict(name=["collplus", "plus"], path=os.path.join(gp, "collPlus"), repo=1),  # you can define multiple name for repo
  dict(name=["Sample", "sample"], path=os.path.join(gp, "sample"), repo=1),
  dict(name="") # it's just dummy
]
```

# How to use

## main ui
* `J/K` - move focus
* `U` - move upper folder
* `H/enter` - enter the folder
* `E` - edit the file
* `T` - register/unregister the folder
* `Alt+Right/Left` - Add or remove the folder in Workspace
* `Alt+Up/Dowm` - Move workspace between folder list

* `a~z` - find folder 
* `/` - command mode - reg / list



## change folder
* Go to a project folder
$ dc coll

* Go to another project folder
$ dc plus

## update all project repositories
$ dc update

** all projects that defined in path.py with repo=1 flag are updated(git fetch + rebase to remote tracking branch + print status)

** `update` command is comprised of `dc fetch` + `dc merge` + `dc st(status)`

## print all project's status
$ dc st

** you can combinate several command as follows,

$ dc fetch st

** you can print current folder or certain repo's status as well

$ dc update .

$ dc update plus

## git CUI helper for add/reset, commit
$ dc ci

** you can use the following keys
* `A` - git add current file
* `P` - prompty for git add -p command
* `R` - reset current staged file
* `D` - drop modification
* `[`/`]` - move next/previous file
* `J`/`K` - scroll down/up - you can use arrow key too
* `C` - popup commit dialog
* `F4`/`Q` - quit the program

### on commit dialog,
** you can see all staged modification for input commit message
* up/down - scroll current file's content
* left/right - prev/next file to see modification

** then input commit message and then enter to do commit


## git push
** you can conveniently push commits just specify target branch name

** push command always check tracking branch firstly than ask you to rebase onto.

$ dc push

** just type target branch you want to push to

# Others
$ dcf "*.py"

$ dcg "printf"
 

