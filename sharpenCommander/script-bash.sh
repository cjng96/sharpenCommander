
# . $DEV_CMD_PATH/bash-script.sh

realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}


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
	python3 $SC_DIR/run.py $@
	goPath
}
function scf()
{
	python3 $SC_DIR/run.py find $@
	goPath
}
function scg()
{
	python3 $SC_DIR/run.py grep $@
	goPath
}
function scw()
{
	python3 $SC_DIR/run.py which $@
	goPath
}

function scd()
{
	python3 -m pudb.run $SC_DIR/run.py $@
	goPath
}

