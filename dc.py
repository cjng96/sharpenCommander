#!/usr/bin/env python3

import subprocess

import tool
from tool import git, system, systemSafe



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
			lst = pp["name"]
			if type(lst) == str:
				lst = [lst] 

			if target.lower() in map(str.lower, lst):
				self.savePath(pp["path"])
				return
				
		raise ExcFail("No that folder[%s]" % target)

	def listPath(self):
		for pp in self.lstPath:
			print(pp)


	def printCommitLogForPush(self, currentBranch, remoteBranch):
		# push 할 커밋 내역 
		gap = git.commitGap(currentBranch, remoteBranch)
		if gap == 0:
			git.printStatus()
			raise ExcFail("There is no commit to push")

		print("There are %d commits to push" % gap)
		ss = git.commitLogBetween(currentBranch, remoteBranch)
		print(ss)
		

	def gitPush(self):
		currentBranch = git.getCurrentBranch()
		remoteBranch = git.getTrackingBranch()
		if remoteBranch == None:
			print("currentBranch:%s DONT have tracking branch")
			# todo: print latest 10 commits

		else:
			print("currentBranch:%s, remote:%s" % (currentBranch, remoteBranch))
			
			self.printCommitLogForPush(currentBranch, remoteBranch)

			# remoteBranch의 fast-forward인지 체크
			rev1 = git.rev(currentBranch)
			rev2 = git.rev("remotes/"+remoteBranch)
			revCommon = git.commonParentRev(currentBranch, remoteBranch)
			if rev2 == revCommon:
				print("local branch is good situation")
			else:
				diffList = git.checkFastForward(currentBranch, remoteBranch)
				if len(diffList) == 0:
					while True:
						hr = input("\n\n*** You can rebase local to remoteBranch. want? y/n: ").lower()
						if hr == 'y':
							ss = git.rebase(remoteBranch)
							# exe result?
							print(ss)
							break
						elif hr == "n":
							break
				else:
					while True:
						hr = input("\n\n*** It could be impossible to rebase onto remoteBranch. rebase/skip: ").lower()
						if hr == 'rebase':
							ss = git.rebase(remoteBranch)
							print(ss)
							break
						elif hr == 'skip':
							break
		
				# print commit log again					
				self.printCommitLogForPush(currentBranch, remoteBranch)
							
		
		git.printStatus()

		target = input("\nInput remote branch name you push to: ")
		if target == "":
			raise ExcFail("Push is canceled")
			

		# push it	
		ss, status = systemSafe("git push origin %s:%s" % (currentBranch, target))
		print(ss)
		
		if status != 0:
			while True:
				hr = input("\n\nPush failed. Do you want to push with force option?[y/N]: ").lower()
				if hr == 'y':
					ss = system("git push origin %s:%s -f" % (currentBranch, target))
					print(ss)				
					break
				elif hr == 'n' or hr == '':
					break
		

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
		
	if not os.path.isfile(os.path.join(pp, "path.py")):
		raise ExcFail("No path.py file in ~/.devcmd")

		
	sys.path.append(pp)
	m = __import__("path")
	g.lstPath = m.pathList
	
	if len(sys.argv) == 1:
		target = "~"
	else:
		target = sys.argv[1]
		
	if target == "push":
		print("fetching first...")
		git.fetch()
		g.gitPush()
		return
	elif target == "list":
		g.listPath()
		return
	elif target == "config":
		g.savePath("~/.devcmd")
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
	

