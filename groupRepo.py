#!/usr/bin/env python3
#-*- coding: utf-8 -*-

import os
import sys
import click
import subprocess
import re


from enum import Enum

import tool
from tool import git, system, systemSafe


'''
$cat ~/bin/ipc

#!/bin/bash
~/gr/gr.py ~/ipc-tool/bin/ipc.py $1 $2 $3


'''

CONTEXT_SETTINGS = dict(help_option_names=['-h', '--help'])

class BlueExcept(Exception):
	def __init__(self, msg):
		super(self, msg)
		

		
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
		return [repo["name"][0] for repo in self.repoList]
		
		
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
			if name in repo["name"]:
				return repo
		raise Exception("Can't find repo[name:%s]" % name)
				
	def changePath(self, name):
		repo = self.getRepo(name)
		path = repo["path"]
		
		if not os.path.isdir(path):
			raise Exception("%s(%s) -> doesn't exist"  % (name, path))

		os.chdir(path)
		ss = "path:%s" % (path)
		return ss
		
				
	def checkSameWith(self, name, branchName, remoteBranch):
		rev = git.rev(branchName)
		rev2 = git.rev("remotes/"+remoteBranch)
		isSame = rev == rev2
		if isSame:
			self.log2(Color.blue, name, "%s is same to %s"  % (branchName, remoteBranch))
			return True
		else:
			commonRev = git.commonParent(branchName, remoteBranch)
			#print("common - %s" % commonRev)
			if commonRev != rev2:
				self.log2(Color.red, name, "%s(%s) - origin/master(%s) -->> Different" % (branchName, rev, rev2))
				return False
		
			# 오히려 앞선경우다. True로 친다.
			gap = git.commitGap(branchName, remoteBracnh)
			self.log2(Color.red, name, "Your local branch(%s) is forward than %s[%d commits]" % (branchName, remoteBranch, gap))
			
			# print commit log
			#ss = system("git log --oneline --graph --all --decorate --abbrev-commit %s..%s" % (remoteBranch, branchName))
			ss = git.commitLogBetween(branchName, remoteBranch)
			print(ss)
			
			return True

	def statusComponent(self, name):
		path = self.changePath(name)
		
		branchName = git.getCurrentBranch()
		remoteBranch = git.getTrackingBranch()
		if remoteBranch == None:
			self.log2(Color.red, name, "%s DONT'T HAVE TRACKING branch" % branchName)
			return
		

		isSame = self.checkSameWith(name, branchName, remoteBranch)
		if isSame:
			# check staged file and untracked file
			ss = system("git status -s")
			if ss != "":
				print(ss)
		else:
			diffList = git.checkFastForward(branchName, remoteBranch)
			if len(diffList) == 0:
				self.log2(Color.blue, name, "Be able to fast-forward... - %s" % path)
			else:
				self.log2(Color.red, name, "NOT be able to fast forward - %s" % path)
			
			#ss = system("git st")
			#print(ss)
			
	def mergeSafe(self, name):
		path = self.changePath(name)

		branchName = git.getCurrentBranch()
		remoteBranch = git.getTrackingBranch()
		if remoteBranch == None:
			self.log2(Color.red, name, "%s DONT'T HAVE TRACKING branch" % branchName)
			return
		
		isSame = self.checkSameWith(name, branchName, remoteBranch)
		if isSame:
			return
	
		diffList = git.checkFastForward(branchName, remoteBranch)
		if len(diffList) != 0:
			self.log2(Color.red, name, "NOT be able to fast forward - %s" % path)
		else:			
			self.log2(Color.blue, name, "merge with %s - %s" % (remoteBranch, path))
			ss = system("git rebase %s" % remoteBranch)
			print(ss)
            
            
            
	def fetch(self, name):
		path = gr.changePath(name)
		self.log2(Color.blue, name, "fetch --prune - %s" % path)
		system("git fetch --prune")


gr = Gr()


@click.group(context_settings=CONTEXT_SETTINGS, chain=True)
@click.version_option(version='1.0.0')
#@click.argument('config')
@click.option('--config')
@click.option('--verbose', type=int, default=0)
def run(config, verbose):
	if config == None:
		config = "~/.devcmd/path.py"

	print("config file: %s" % config)
	if verbose > 0:
		tool.g.isPrintSystem = True
	
	#cur = os.getcwd()
	cur = os.path.dirname(config)
	name = os.path.basename(config)
	cur = os.path.expanduser(cur)
	#print("current path: %s - %s" % (cur, name))
	sys.path.append(cur)
	name = os.path.splitext(name)[0]
	
	op = __import__(name)
	gr.repoList = [repo for repo in op.pathList if "repo" in repo and repo["repo"] != 0]
	for repo in gr.repoList:
		repo["path"] = os.path.expanduser(repo["path"])
		name = repo["name"]
		if type(name) is str:
			repo["name"] = [name]

@run.command('update', help='fetch + merge + status')
@click.pass_context
def cmdUpdate(ctx):
	print("fetch.......")
	ctx.invoke(cmdFetch)
	print("\nmerge.......")
	ctx.invoke(cmdMerge)
	print("\nstatus......")
	ctx.invoke(cmdStatus, component="")


@run.command('st', help='status of all or indicated component')
@click.argument('component', nargs=-1)
def cmdStatus(component):
	for comp in gr.repoAllName():
		gr.statusComponent(comp)

@run.command('fetch', help="fetch all component with prune option")
@click.pass_context
def cmdFetch(ctx):
	for comp in gr.repoAllName():
		gr.fetch(comp)
	# we support command chaning
	#gr.log(2, "\nautomatic status...")	
	#ctx.invoke(cmdStatus, component="")

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


