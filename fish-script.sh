
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

function sc
	eval $DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/sc.py $argv
	goPath
end
		
function scf
	eval $DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/sc.py find $argv
	goPath
end

function scg
	eval $DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/sc.py grep $argv
	goPath
end

function scw
	eval $DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/sc.py which $argv
	goPath
end

function scd
	eval $DEV_CMD_DIR/env/bin/python3 -m pudb.run $DEV_CMD_DIR/sc.py $argv
	goPath
end

