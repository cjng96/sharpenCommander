
# DEV_CMD_PATH=~/devCmdTool
# . $DEV_CMD_PATH/bash-script.sh

set DEV_CMD_DIR (realpath (dirname (status -f)))

function goPath
	# goto that path
	if test -e /tmp/cmdDevTool.path
		set -l DEVPP (cat /tmp/cmdDevTool.path)
		rm -f /tmp/cmdDevTool.path
		cd $DEVPP
	end
end

function dc
    echo $PP
	python3 $DEV_CMD_DIR/dc.py $argv
	goPath
end
		
function dcf
	python3 $DEV_CMD_DIR/dc.py find $argv
	goPath
end

function dcg
	python3 $DEV_CMD_DIR/dc.py grep $argv
	goPath
end

function dcw
	python3 $DEV_CMD_DIR/dc.py which $argv
	goPath
end

function dcd
	python3 -m pudb.run $DEV_CMD_DIR/dc.py $argv
	goPath
end

#alias dc="$DEV_CMD_PATH/dc.py"
