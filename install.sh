#!/bin/bash

TARGET=/usr/local/src
mkdir -p $TARGET
cd $TARGET

if [ ! -d devCmdTool ]; then
    git clone -b stable https://github.com/cjng96/devCmdTool.git
else
    echo "There is already devCmdTool repo in /usr/local/src"
fi

COMMENT="## devCmdTool script ##"
cnt=$(sh -c "grep '$COMMENT' ~/.bashrc | wc -l")
if [ $cnt -eq  0 ]; then
    echo "Setting up for devCmdTool to ~/.bashrc"
    echo $COMMENT >> ~/.bashrc
    echo ". $TARGET/bash-script.sh" >> ~/.bashrc
    echo "Please type 'dc' after restarting terminal or source ~/.bashrc"
else
    echo "Setting is done already. Type 'dc' for starting"
fi


