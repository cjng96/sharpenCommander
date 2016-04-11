

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



## install 

mkdir ~/work_inertry
git clone git@github.com:cjng96/devCmdTool.git

cd ~
mkdir bin
ln -s ~/work_inertry/groupRepo.py ~/bin/iner


cd ~
vi .bashrc

DEV_CMD_PATH=~/devCmdTool
. $DEV_CMD_PATH/bash-script.sh


