

# groupRepo
Group of repo management tool

# dc
* change folder
* run git command


# how to install

## environment

apt-get install python3-setuptools
easy_install-3.4 pip

pip3 install click
pip3 install urwid


## install 

mkdir ~/work
git clone git@github.com:cjng96/devCmdTool.git

cd ~
mkdir bin
ln -s ~/work/groupRepo.py ~/bin/iner


cd ~
vi .bashrc

DEV_CMD_PATH=~/devCmdTool
. $DEV_CMD_PATH/bash-script.sh

mkdir ~/.devcmd

** write your path.py file as follows

pathList = [
        dict(name="dev", path=os.path.join(g_root, "devCmdTool"), repo=1),
        dict(name=["coCpp", "cpp"], path=os.path.join(g_root, "coCpp"), repo=1),
        dict(name="")
]

  