
# . $DEV_CMD_PATH/bash-script.sh

set SC_DIR (realpath (dirname (status -f)))
set -x SC_OK 1

function goPath
	# goto that path
	if test -e /tmp/cmdDevTool.path
		set -l DEVPP (cat /tmp/cmdDevTool.path)
		rm -f /tmp/cmdDevTool.path
		cd $DEVPP
	end
end

function sc
	python3 $SC_DIR/run.py $argv
	goPath
end
function scf
	python3 $SC_DIR/run.py find $argv
	goPath
end
function scg
	python3 $SC_DIR/run.py grep $argv
	goPath
end
function scw
	python3 $SC_DIR/run.py which $argv
	goPath
end

function scd
	python3 -m pudb.run $SC_DIR/run.py $argv
	goPath
end

