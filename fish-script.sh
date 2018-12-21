
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
	eval $DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/dc.py $argv
	goPath
end
		
function dcf
	eval $DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/dc.py find $argv
	goPath
end

function dcg
	eval $DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/dc.py grep $argv
	goPath
end

function dcw
	eval $DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/dc.py which $argv
	goPath
end

function dcd
	eval $DEV_CMD_DIR/env/bin/python3 -m pudb.run $DEV_CMD_DIR/dc.py $argv
	goPath
end

