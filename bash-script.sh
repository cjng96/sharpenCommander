
# DEV_CMD_PATH=~/devCmdTool
# . $DEV_CMD_PATH/bash-script.sh

DEV_CMD_DIR=$(dirname "$0")
DEV_CMD_DIR=$(realpath "$PP")

function goPath()
{
	# goto that path
	if [ -f /tmp/cmdDevTool.path ]; then
		DEVPP=$(cat /tmp/cmdDevTool.path)
		rm -f /tmp/cmdDevTool.path
		cd $DEVPP
	fi

}
function dc()
{
	$DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/dc.py "$1" "$2" "$3" "$4" "$5" "$6" "$7" "$8" "$9"
	goPath
		
}
function dcf()
{
	$DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/dc.py find "$1" "$2" "$3" "$4" "$5"
	goPath
}
function dcg()
{
	$DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/dc.py grep "$1" "$2" "$3" "$4" "$5"
	goPath
}
function dcw()
{
	$DEV_CMD_DIR/env/bin/python3 $DEV_CMD_DIR/dc.py which "$1" "$2" "$3" "$4" "$5"
	goPath
}

function dcd()
{
	$DEV_CMD_DIR/env/bin/python3 -m pudb.run $DEV_CMD_DIR/dc.py "$1" "$2" "$3" "$4" "$5" "$6" "$7" "$8" "$9"
	goPath
}


#alias dc="$DEV_CMD_PATH/dc.py"
