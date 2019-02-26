
# DEV_CMD_PATH=~/devCmdTool
# . $DEV_CMD_PATH/bash-script.sh

DEV_CMD_DIR=$(realpath $(dirname "${BASH_SOURCE[0]:-${(%):-%x}}"))

function goPath()
{
	# goto that path
	if [ -f /tmp/cmdDevTool.path ]; then
		DEVPP=$(cat /tmp/cmdDevTool.path)
		rm -f /tmp/cmdDevTool.path
		cd $DEVPP
	fi

}
function sc()
{
	$DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/sc.py "$1" "$2" "$3" "$4" "$5" "$6" "$7" "$8" "$9"
	goPath
		
}
function scf()
{
	$DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/sc.py find "$1" "$2" "$3" "$4" "$5"
	goPath
}
function scg()
{
	$DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/sc.py grep "$1" "$2" "$3" "$4" "$5"
	goPath
}
function scw()
{
	$DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/sc.py which "$1" "$2" "$3" "$4" "$5"
	goPath
}

function scd()
{
	$DEV_CMD_DIR/env/bin/python3 -m pudb.run $DEV_CMD_DIR/sc.py "$1" "$2" "$3" "$4" "$5" "$6" "$7" "$8" "$9"
	goPath
}


#alias dc="$DEV_CMD_PATH/dc.py"
