# coding: utf-8
#!/usr/bin/env python3

import subprocess

import sys
import tool
import pudb
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
		# commit log to push
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

			# check if fast-forward of remoteBranch
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


import urwid
import urwid.raw_display
import urwid.web_display
from urwid.signals import connect_signal

class mButton(urwid.Button):
	'''
	Button without pre/post Text
	'''
	def __init__(self, label, on_press=None, user_data=None):
		self._label = urwid.wimp.SelectableIcon(label, 0)
		super(urwid.Button, self).__init__(self._label)
		#urwid.widget.WidgetWrap.__init__(self, self._label)

		# The old way of listening for a change was to pass the callback
		# in to the constructor.  Just convert it to the new way:
		if on_press:
			connect_signal(self, 'click', on_press, user_data)

		#self.set_label(label)

class mListBox(urwid.ListBox):
	def focusNext(self):
		try: 
			self.body.set_focus(self.body.get_next(self.body.get_focus()[1])[1])
		except:
			pass
			
	def focusPrevious(self):
		try: 
			self.body.set_focus(self.body.get_prev(self.body.get_focus()[1])[1])
		except:
			pass      

	# TODO: scroll 
	def scrollDown(self):
		cur = self.body.get_focus()
		if cur[1] >= len(self.body)-1:
			return
			
		nextRow = self.body.get_next(cur[1])
		self.body.set_focus(nextRow[1])
			
	def scrollUp(self):
		cur = self.body.get_focus()
		if cur[1] == 0:
			return
			
		self.body.set_focus(self.body.get_prev(cur[1])[1])


class Urwid:

	def terminal2markup(ss):
		#source = "\033[31mFOO\033[0mBAR"
		table = {"[1":'blod', "[31":'redfg', "[32":'greenfg', "[33":'yellowfg', "[36":'cyanfg', "[41":"redbg", "[0":'std', "[":'reset'}
		markup = []
		st = ss.find("\x1b")
		if st == -1:
			return ss
			
		items = ss.split("\x1b")
		pt = 1
		if not ss.startswith("\x1b"):
			markup.append(items[0])
		
		for at in items[pt:]:
			attr, text = at.split("m",1)
			if text != "":	# skip empty string
				markup.append((table[attr], text))
			
		return markup
		
	def genEdit(label, text, fn):
		w = urwid.Edit(label, text)
		urwid.connect_signal(w, 'change', fn)
		fn(w, text)
		w = urwid.AttrWrap(w, 'edit')
		return w
		
		
	def makeTextList(lstStr):
		outList = []
		for line in lstStr:
			line2 = Urwid.terminal2markup(line)
			#g.log.write("ma - %s\n -> %s\n" % (line, line2))
			outList.append(urwid.Text(line2))
		return outList
		
	def makeBtnList(lstStr, onClick):
		outList = []
		for line in lstStr:
			btn = mButton(line, onClick)
			outList.append(btn)
		return outList

def unhandled(key):
	if key == 'f8' or key == "q":
		raise urwid.ExitMainLoop()
	elif key == "up" or key == 'k':
		g.widgetContent.scrollDown()
	elif key == "down" or key == 'j':
		g.widgetContent.scrollUp()
	elif key == "left":
		pass
	elif key == "right":
		pass
		
def inputFilter(keys, raw):
	return keys

def onFileSelected(btn):
	label = btn.get_label()
	fileName = label[2:].strip()

	g.headerText.set_text("file - " + label)
	
	# display
	if label.startswith("?? "):
		ss = open(fileName, "rb").read().decode()
	else:
		ss = system("git diff --color %s" % fileName)
		
	ss = ss.replace("\t", "    ")
		
	del g.widgetContent.body[:]
	g.widgetContent.body += Urwid.makeTextList(ss.split("\n"))
	g.widgetContent.set_focus(0)
	

def urwidGitStatus():
	#lst = system("git -c color.status=always status")
	lst = system("git status -s")
	lstContent = ["test"]

	fileList = mListBox(urwid.SimpleListWalker(Urwid.makeBtnList(lst.split("\n"), onFileSelected)))
	g.widgetContent = mListBox(urwid.SimpleListWalker(Urwid.makeTextList(lstContent)))
	g.widgetMain = urwid.Pile([(10, urwid.AttrMap(fileList, 'std')), g.widgetContent])
	
	g.headerText = urwid.Text("header...")
	frame = urwid.Frame(g.widgetMain, header=g.headerText)
		
	# (name, fg, bg, mono, fgHigh, bgHigh)
	palette = [
		('std', 'light gray', 'black'),
		('reset', 'std'),
		('blod', 'light gray,bold', 'black'),
		('redfg', 'dark red', 'black'),
		('greenfg', 'dark green', 'black'),
		('yellowfg', 'yellow', 'black'),
		('bluefg', 'dark blue', 'black'),
		('cyanfg', 'dark cyan', 'black'),
		
		('redbg', 'black', 'dark red'),
		
		('body','black','light gray', 'standout'),
		('reverse','light gray','black'),
		('header','white','dark red', 'bold'),
		('important','dark blue','light gray',('standout','underline')),
		('editfc','white', 'dark blue', 'bold'),
		('editbx','light gray', 'dark blue'),
		('editcp','black','light gray', 'standout'),
		('bright','dark gray','light gray', ('bold','standout')),
		('buttn','black','dark cyan'),
		('buttnf','white','dark blue','bold'),
		]
		
	# use appropriate Screen class
	#if urwid.web_display.is_web_request():
	#	screen = urwid.web_display.Screen()
	#else:
	#	screen = urwid.raw_display.Screen()
	screen = urwid.raw_display.Screen()

	urwid.MainLoop(frame, palette, screen,
		unhandled_input=unhandled, input_filter=inputFilter).run()
		
		

g = Global()
g.log = open("log.log", "w", encoding="UTF-8")

def winTest():
	ss = system("c:\\cygwin64\\bin\\git.exe diff --color dc.py")

	kk = Urwid.terminal2markup(ss)
	st = ss.find("\x1b")
	print("%d %x %x %x %x" % (st, ss[0], ss[1], ss[2], ss[3]))
	sys.exit(0)


def run():
	#winTest()
	
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
	elif target == "st":
		urwidGitStatus()
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
		# no working..
		f = open("err.log", "w")
		original_stderr = sys.stdout
		sys.stdout = f
		
		ret = run()
	except ExcFail as e:
		print(e)
		sys.exit(1)
	

