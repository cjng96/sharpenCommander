import os
import sys
import re
import subprocess

class Config:
	def __init__(self):
		self.isPrintSystem = False
	
g = Config()


def system(args, stderr=subprocess.STDOUT):
	if g.isPrintSystem:
		print("system command - %s" % args)
	rr = subprocess.check_output(args, stderr=stderr, shell=True).decode("UTF-8")
	rr = rr.rstrip(' \r\n')
	return rr

def systemSafe(args):
	if g.isPrintSystem:
		print("system command - %s" % args)
	status,output = subprocess.getstatusoutput(args)
	#rr = output.decode("UTF-8")
	rr = output
	rr = rr.strip(' \r\n')
	return rr,status

def systemRet(args):
	if g.isPrintSystem:
		print("system command - %s" % args)
		
	ret = subprocess.call(args, shell=True)
	return ret


def programPath(sub=None):
	pp = os.path.dirname(os.path.realpath(sys.argv[0]))
	if sub is not None:
		pp = os.path.join(pp, sub)
	return pp


class git:
	# if remote branch, insert "remotes/"
	@staticmethod
	def rev(branch):
		ss = system("git branch -va")
		m = re.search(r'^[*]?\s+%s\s+(\w+)' % branch, ss, re.MULTILINE)
		rev = m.group(1)
		return rev

	@staticmethod
	def getCurrentBranch():
		return system("git rev-parse --abbrev-ref HEAD")

	@staticmethod
	def getTrackingBranch():
		try:
			return system("git rev-parse --abbrev-ref --symbolic-full-name @{u}")
		except subprocess.CalledProcessError:
			return None

	@staticmethod
	def commonParentRev(br1, br2):
		commonRev = system("git merge-base %s %s" % (br1, br2))
		return commonRev[:7]

	@staticmethod
	def printStatus():
		ss = system("git -c color.status=always status -s")
		print("\n"+ss+"\n")
		

	@staticmethod
	def commitGap(brNew, brOld):
		gap = system("git rev-list %s ^%s --count" % (brNew, brOld))
		return int(gap)

	@staticmethod
	def commitLogBetween(brNew, brOld):
		# color print
		ss = system("git log --color --oneline --graph --decorate --abbrev-commit %s^..%s" % (brOld, brNew))
		return ss
		

	@staticmethod
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

	@staticmethod
	def fetch():
		return system("git fetch --prune")
		
	@staticmethod
	def rebase(branch):
		return systemSafe("git rebase %s" % branch)
	
	@staticmethod
	def stashGetNameSafe(name):
		ss = system("git stash list")
		print(ss)
		m = re.search(r'^(stash@\{\d+\}):\s(\w|\s).+: %s$' % name, ss)
		if not m:
			return None

		return m.group(1)
	
	@staticmethod
	def stashPop(name):
		ss = system("git stash pop %s" % name)
		print
		
	@staticmethod
	def statusFileList():
		"""
		file list(staged, modified) in current folder by terminal character
		(terminal name, s or "")
		:return:
		"""
		fileList = system("git -c color.status=always status -s", stderr=subprocess.DEVNULL)

		# quoted octal notation to utf8
		fileList = bytes(fileList, "utf-8").decode("unicode_escape")
		bb = fileList.encode("ISO-8859-1")
		fileList = bb.decode()

		# remove "" in file name
		fileList2 = []
		for line in fileList.splitlines():
			fileType, fileName = line.split(" ", 1)
			if fileName.startswith("\"") and fileName.endswith("\""):
				fileName = fileName[1:-1]
			fileList2.append(fileType + " " + fileName)

		def getStatus(terminal):
			if "[32m" in terminal:
				return "s"
			elif "??" in terminal:
				return "?"
			else:   # modification
				return ""

		itemList = [(x, getStatus(x)) for x in fileList2 if len(x) > 0]
		return itemList