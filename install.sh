#!/bin/bash

TARGET=~/.synapcmd
mkdir -p $TARGET
cd $TARGET
REPO=$TARGET/synapbookCommander

if [[ ! -d synapbookCommander ]]; then
    git clone -b stable https://github.com/inertry/synapbookCommander.git
else
    echo "There is already SynapbookCommander repo in $TARGET"
fi

COMMENT="## SynapbookCommander script ##"
cnt=$(sh -c "grep '$COMMENT' ~/.bashrc | wc -l")
if [[ $cnt -eq  0 ]]; then
    cd synapbookCommander
    [ $? -ne 0 ] && echo "#### no repo folder" && exit
    virtualenv -p python3 env
    [ $? -ne 0 ] && echo "#### failed to run virtualenv" && exit
    ./env/bin/pip3 install -r requirements.txt
    [ $? -ne 0 ] && echo "#### failed to install python components" && exit

    echo "Setting up for SynapbookCommander to ~/.bashrc"
    echo $COMMENT >> ~/.bashrc
    echo ". $REPO/bash-script.sh" >> ~/.bashrc
    echo "Please type 'sc' after restarting terminal or source ~/.bashrc"
else
    echo "Setting is done already. Type 'sc' for starting"
fi

