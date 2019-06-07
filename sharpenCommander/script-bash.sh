
# . $DEV_CMD_PATH/bash-script.sh

SC_DIR=$(realpath $(dirname "${BASH_SOURCE[0]:-${(%):-%x}}"))
export SC_OK=1
function goPath()
{
	# goto that path
	if [ -f /tmp/cmdDevTool.path ]; then
		PP=$(cat /tmp/cmdDevTool.path)
		rm -f /tmp/cmdDevTool.path
		cd $PP
	fi

}
function sc()
{
	/usr/bin/env python3 $SC_DIR/sc.py $@
	goPath
}
function scf()
{
	/usr/bin/env python33 $SC_DIR/sc.py find $@
	goPath
}
function scg()
{
	/usr/bin/env python3 $SC_DIR/sc.py grep $@
	goPath
}
function scw()
{
	/usr/bin/env python3 $SC_DIR/sc.py which $@
	goPath
}

function scd()
{
	/usr/bin/env python3 -m pudb.run $SC_DIR/sc.py $@
	goPath
}

