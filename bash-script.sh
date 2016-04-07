
# DEV_CMD_PATH=~/devCmdTool
# . DEV_CMD_PATH/bash-script.sh


function dc()
{
	python3 $DEV_CMD_PATH/dc.py $1 $2 $3
	if [ $? != 0 ]; then
		return
	fi
		
	# goto that path
	DEVPP=$(cat /tmp/cmdDevTool.path)
	rm -f /tmp/cmdDevTool.path
	cd $DEVPP
}

#alias dc="$DEV_CMD_PATH/dc.py"
