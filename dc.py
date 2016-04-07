#!/usr/bin/env python3

import subprocess

'''
dc - devCmd

# install setting

1. append some lines to .bashrc as follows

  DEV_CMD_PATH=~/devCmdTool
  . DEV_CMD_PATH/bash-script.sh

2. write devPath.py file on ~/.devcmd

  G_PATH_LIST = [
        dict(name="ipc", path="~/ipc-linux")
  ]


# usage

1. push command
 1) print git status
 2) input target branch name
 3) git push origin master:TARGET_BRANCH
 
 

'''

class ExcFail(Exception):
	def __init__(self, msg):
		super().__init__(msg)
		

def system(args):
	if g.isPrintSystem:
		print("system command - %s" % args)
	rr = subprocess.check_output(args, shell=True).decode("UTF-8")
	rr = rr.strip(' \r\n')
	return rr		

import os, sys

class Global:
	def __init__(self):
		self.lstPath = []
		self.isPrintSystem = False
		
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
		
	def gitPush(self):
		currentBranch = system("git rev-parse --abbrev-ref HEAD")
		remoteBranch = "origin/master"
		print("currentBranch:%s, remote:%s" % (currentBranch, remoteBranch))

		ss = system("git status -s")
		print("\n"+ss+"\n")

		gap = system("git rev-list %s ^%s --count" % (currentBranch, remoteBranch))
		gap = int(gap)
		if gap == 0:
			raise ExcFail("There is no commit to push")

		print("There are %d commits to push" % gap)
	
		ss = system("git log --oneline --graph --decorate --abbrev-commit %s^..%s" % (remoteBranch, currentBranch))
		print(ss)
		
		
		target = input("\nInput remote branch name you push to : ")
		if target == "":
			raise ExcFail("Push is canceled")
			

		# push it			
		ss = system("git push origin %s:%s" % (currentBranch, target))
		print(ss)
		

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
		
	if target == "push":
		g.gitPush()
		return
		
	#print("target - %s" % target)
	g.cd(target)
	return 1
	

if __name__ == "__main__":
	try:
		ret = run()
	except ExcFail as e:
		print(e)
		sys.exit(1)
	

