
# DEV_CMD_PATH=~/devCmdTool
# . $DEV_CMD_PATH/bash-script.sh


function dc()
{
	python3 $DEV_CMD_PATH/dc.py $1 $2 $3
		
	# goto that path
	if [ -f /tmp/cmdDevTool.path ]; then
		DEVPP=$(cat /tmp/cmdDevTool.path)
		rm -f /tmp/cmdDevTool.path
		cd $DEVPP
	fi
}


function dcd()
{
	python3 -m pudb.run $DEV_CMD_PATH/dc.py $1 $2 $3
		
	# goto that path
	if [ -f /tmp/cmdDevTool.path ]; then
		DEVPP=$(cat /tmp/cmdDevTool.path)
		rm -f /tmp/cmdDevTool.path
		cd $DEVPP
	fi
}


#alias dc="$DEV_CMD_PATH/dc.py"
