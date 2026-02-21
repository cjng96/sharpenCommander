# . $DEV_CMD_PATH/bash-script.sh

realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}


SCR_DIR=$(realpath $(dirname "${BASH_SOURCE[0]:-${(%):-%x}}"))
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
function sc2()
{
	PP=$(pwd)
	cd $SCR_DIR
	cargo run -- --path=$PP $@
	goPath
}

function sc()
{
	~/bin/sc $@
	goPath
}
