import subprocess
import re

class Config:
	def __init__(self):
		self.isPrintSystem = False
	
g = Config()


def system(args):
	if g.isPrintSystem:
		print("system command - %s" % args)
	rr = subprocess.check_output(args, shell=True).decode("UTF-8")
	rr = rr.strip(' \r\n')
	return rr

def systemSafe(args):
	if g.isPrintSystem:
		print("system command - %s" % args)
	status,output = subprocess.getstatusoutput(args)
	#rr = output.decode("UTF-8")
	rr = output
	rr = rr.strip(' \r\n')
	return rr,status


class git:
	# if remote branch, insert "remotes/"
	def rev(branch):
		ss = system("git branch -va")
		m = re.search(r'^[*]?\s+%s\s+(\w+)' % branch, ss, re.MULTILINE)
		rev = m.group(1)
		return rev

	def getCurrentBranch():
		return system("git rev-parse --abbrev-ref HEAD")

	def getTrackingBranch():
		try:
			return system("git rev-parse --abbrev-ref --symbolic-full-name @{u}")
		except subprocess.CalledProcessError:
			return None

	def commonParentRev(br1, br2):
		commonRev = system("git merge-base %s %s" % (br1, br2))
		return commonRev[:7]

	def printStatus():
		ss = system("git status -s")
		print("\n"+ss+"\n")
		

	def commitGap(brNew, brOld):
		gap = system("git rev-list %s ^%s --count" % (brNew, brOld))
		return int(gap)

	def commitLogBetween(brNew, brOld):
		ss = system("git log --oneline --graph --decorate --abbrev-commit %s^..%s" % (brOld, brNew))
		return ss
		

	def checkFastForward( br1, br2):
		commonRev = git.commonParentRev(br1, br2)
		
		br1Diff = system("git diff --name-only %s %s" % (commonRev, br1))
		br2Diff = system("git diff --name-only %s %s" % (commonRev, br2))
		
		br1 = br1Diff.split()
		br2 = br2Diff.split()
		
		# check same file
		lst2 = []
		for ss in br1:
			if ss in br2:
				lst2.append(ss)
				
		return lst2

	def fetch():
		return system("git fetch --prune")
		
	def rebase(branch):
		return system("git rebase %s" % branch)
	
	def stashGetNameSafe(name):
		ss = system("git stash list")
		print(ss)
		m = re.search(r'^(stash@\{\d+\}):\s(\w|\s).+: %s$' % name, ss)
		if not m:
			return None

		return m.group(1)
	
	def stashPop(name):
		ss = system("git stash pop %s" % name)
		print
		
		
