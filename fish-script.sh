
# DEV_CMD_PATH=~/devCmdTool
# . $DEV_CMD_PATH/bash-script.sh


function goPath
	# goto that path
	if test -e /tmp/cmdDevTool.path
		set -l DEVPP (cat /tmp/cmdDevTool.path)
		rm -f /tmp/cmdDevTool.path
		cd $DEVPP
	end
end

function dc
	python3 $DEV_CMD_PATH/dc.py $argv
	goPath
end
		
function dcf
	python3 $DEV_CMD_PATH/dc.py findg $argv
	goPath
end

function dcg
	python3 $DEV_CMD_PATH/dc.py ackg $argv
	goPath
end

function dcw
	python3 $DEV_CMD_PATH/dc.py which $argv
	goPath
end

function dcd
	python3 -m pudb.run $DEV_CMD_PATH/dc.py $argv
	goPath
end

#alias dc="$DEV_CMD_PATH/dc.py"
