#!/usr/bin/env python3
'''
dc - devCmd

1. append some lines to .bashrc as follows

DEV_CMD_PATH=~/devCmdTool
. DEV_CMD_PATH/bash-script.sh

2. write devPath.py file on ~/.devcmd

G_PATH_LIST = [
        dict(name="ipc", path="~/ipc-linux")
]


'''

class ExcFail(Exception):
	def __init__(self, msg):
		super().__init__(msg)

import os, sys

class Global:
	def __init__(self):
		self.lstPath = []
		
	def savePath(self, pp):
		with open("/tmp/cmdDevTool.path", "wb") as f:
			f.write(os.path.expanduser(pp).encode())
			
		
	def cd(self, target):
		if target == "~":
			self.savePath(target)
			return
	
		for pp in self.lstPath:
			if pp["name"] == target:
				self.savePath(pp["path"])
				return
				
		raise ExcFail("No that folder[%s]" % target)

g = Global()

def run():
	try:
		os.remove("/tmp/cmdDevTool.path")
	except OSError:
		pass
		
	pp = os.path.expanduser("~/.devcmd")
	if not os.path.isdir(pp):
		print("No .devcmd folder. generate it...")
		os.mkdir(pp)
		
	if not os.path.isfile(os.path.join(pp, "devPath.py")):
		raise ExcFail("No path.py file in ~/.devcmd")

		
	sys.path.append(pp)
	m = __import__("devPath")
	g.lstPath = m.G_PATH_LIST
	
	if len(sys.argv) == 1:
		target = "~"
	else:
		target = sys.argv[1]
		
	#print("target - %s" % target)
	g.cd(target)
	return 1
	

if __name__ == "__main__":
	try:
		ret = run()
	except ExcFail as e:
		print(e)
		sys.exit(1)
	

