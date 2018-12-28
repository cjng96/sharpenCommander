#!/bin/bash

TARGET=~/.synapcmd
mkdir -p $TARGET
cd $TARGET

if [[ ! -d devCmdTool ]]; then
    git clone -b stable https://github.com/inertry/synapbookCommander.git
else
    echo "There is already SynapbookCommander repo in $TARGET"
fi

COMMENT="## SynapbookCommander script ##"
cnt=$(sh -c "grep '$COMMENT' ~/.bashrc | wc -l")
if [[ $cnt -eq  0 ]]; then
    echo "Setting up for SynapbookCommander to ~/.bashrc"
    echo $COMMENT >> ~/.bashrc
    echo ". $TARGET/bash-script.sh" >> ~/.bashrc
    echo "Please type 'dc' after restarting terminal or source ~/.bashrc"
else
    echo "Setting is done already. Type 'sc' for starting"
fi

cd synapbookCommander
virtualenv env
./env/bin/pip3 install -r requirements.txt


