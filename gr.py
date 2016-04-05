#!/usr/bin/env python3
#-*- coding: utf-8 -*-

import os
import sys
import click
import subprocess
import re


from enum import Enum


'''
$cat ~/bin/ipc

#!/bin/bash
~/gr/gr.py ~/ipc-tool/bin/ipc.py $1 $2 $3


'''

CONTEXT_SETTINGS = dict(help_option_names=['-h', '--help'])

class BlueExcept(Exception):
	def __init__(self, msg):
		super(self, msg)
		

def system(args):
  rr = subprocess.check_output(args, shell=True).decode("UTF-8")
  rr = rr.strip('\n')
  return rr

def gitRev(branch):
  ss = system("git br -va")
  m = re.search(r'^[*]?\s+%s\s+(\w+)' % branch, ss, re.MULTILINE)
  rev = m.group(1)
  return rev
		
Color = Enum('color', 'blue red')

class Ansi:
	redBold = "\033[1;31m"
	red = "\033[0;31m"
	blueBold = "\033[1;34m"
	blue = "\033[0;34m"
	clear = "\033[0m"

class Gr:
	def __init__(self):
		self.repoList = dict(name="test", path="")
		
	def repoAllName(self):
		return [repo["name"] for repo in self.repoList]
		
		
	def log(self, lv, msg):
		if lv == 0:
			print("%s%s%s" % (Ansi.redBold, msg, Ansi.clear))
		elif lv == 1:
			print("%s%s%s" % (Ansi.blueBold, msg, Ansi.clear))
		else:
			print("%s" % (msg))
			
	def log2(self, color, name, msg):
		ansiBold = Ansi.blueBold if Color.blue == color else Ansi.redBold
		ansiNor = Ansi.blue if Color.blue == color else Ansi.red
		print("%s%s -> %s%s%s" % (ansiBold, name, ansiNor, msg, Ansi.clear))

	def getRepo(self, name):
		for repo in self.repoList:
			if repo["name"] == name:
				return repo
		raise Exception("Can't find repo[name:%s]" % name)
				
	def changePath(self, name):
		repo = self.getRepo(name)
		path = repo["path"]
		
		if not os.path.isdir(path):
			raise Exception("%s -> doesn't exist"  % (name))

		self.log(2, "\nChange to %s(%s)..." % (name, path))
		os.chdir(path)
		return True
		
	def checkFastForward(self, br1, br2):
		commonRev = system("git merge-base %s %s" % (br1, br2))
		
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
		
	def checkSameWith(self, branchName, remoteBranch):
		rev = gitRev(branchName)
		rev2 = gitRev("remotes/"+remoteBranch)
		isSame = rev == rev2
		if isSame:
			self.log2(Color.blue, branchName, "%s is same to %s"  % (branchName, remoteBranch))
		else:
			commonRev = system("git merge-base %s %s" % (branchName, remoteBranch))
			print("common - %s" % commonRev)
			if commonRev[:7] == rev2:
				# 오히려 앞선경우다. True로 친다.
				gap = system("git rev-list %s ^%s --count" % (branchName, remoteBranch))
				gap = int(gap)
				self.log2(Color.red, branchName, "Your local branch is forward than %s[%d commits]" % (remoteBranch, gap))
				
				# print commit log
				ss = system("git log --oneline --graph --all --decorate --abbrev-commit %s..%s" % (remoteBranch, branchName))
				print(ss)
				
				return True
		
			self.log2(Color.red, branchName, "\t%s:%s - origin/master:%s -->> Different" % (branchName, rev, rev2))
			
		return isSame

	def statusComponent(self, name):
		if not self.changePath(name):
			return 

		branchName = system('git rev-parse --abbrev-ref HEAD')
		originBranch = 'origin/master'
		
		isSame = self.checkSameWith(branchName, originBranch)
		if isSame:
			return
		else:
			diffList = self.checkFastForward(branchName, originBranch)
			if len(diffList) == 0:
				self.log2(Color.blue, name, "Be able to fast-forward...")
			else:
				self.log2(Color.red, name, "NOT be able to fast forward")
			
			#ss = system("git st")
			#print(ss)
			
	def mergeSafe(self, name):
		if not self.changePath(name):
			return 

		branchName = system('git rev-parse --abbrev-ref HEAD')
		originBranch = 'origin/master'
		
		isSame = self.checkSameWith(branchName, originBranch)
		if isSame:
			return
	
		diffList = self.checkFastForward(branchName, originBranch)
		if len(diffList) != 0:
			self.log2(Color.red, name, "NOT be able to fast forward")
		else:			
			self.log2(Color.blue, name, "merge with %s" % originBranch)
			ss = system("git merge %s" % originBranch)
			print(ss)

gr = Gr()


@click.group(context_settings=CONTEXT_SETTINGS)
@click.version_option(version='1.0.0')
@click.argument('config')
def run(config):
	print("config file: %s" % config)
	
	#cur = os.getcwd()
	cur = os.path.dirname(config)
	name = os.path.basename(config)
	print("current path: %s - %s" % (cur, name))
	sys.path.append(cur)
	name = os.path.splitext(name)[0]
	
	op = __import__(name)
	gr.repoList = op.repoList	

	pass


@run.command('st', help='status of all or indicated component')
@click.argument('component', nargs=-1)
def cmdStatus(component):
	for comp in gr.repoAllName():
		gr.statusComponent(comp)

@run.command('fetch', help="fetch all component with prune option")
def cmdFetch():
	for comp in gr.repoAllName():
		if not gr.changePath(comp):
			continue

	system("git fetch --prune")

@run.command('merge', help="merge all componet that can be fast-forward merge")
def cmdMerge():
	for comp in gr.repoAllName():
		gr.mergeSafe(comp)


import sys
import importlib

def main():
	run()


if __name__ == "__main__":
	main()


