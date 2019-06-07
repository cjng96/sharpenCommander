
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
	eval /usr/bin/env python3 $SC_DIR/sc.py $argv
	goPath
end
function scf
	eval /usr/bin/env python3 $SC_DIR/sc.py find $argv
	goPath
end
function scg
	eval /usr/bin/env python3 $SC_DIR/sc.py grep $argv
	goPath
end
function scw
	eval /usr/bin/env python3 $SC_DIR/sc.py which $argv
	goPath
end

function scd
	eval /usr/bin/env python3 -m pudb.run $SC_DIR/sc.py $argv
	goPath
end

