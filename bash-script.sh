
# DEV_CMD_PATH=~/devCmdTool
# . $DEV_CMD_PATH/bash-script.sh


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
	python3 $DEV_CMD_PATH/dc.py "$1" "$2" "$3" "$4" $5 $6 $7 $8 $9
	goPath
		
}
function dcf()
{
	python3 $DEV_CMD_PATH/dc.py findg "$1" "$2"
	goPath
}


function dcd()
{
	python3 -m pudb.run $DEV_CMD_PATH/dc.py $1 $2 $3 $4 $5 $6 $7 $8 $9
	goPath
}


#alias dc="$DEV_CMD_PATH/dc.py"
