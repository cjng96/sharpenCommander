# coding: utf-8
#!/usr/bin/env python3

import subprocess

import os
import sys

import tool
from tool import git, system, systemSafe

import urwid
import urwid.raw_display
import urwid.web_display
from urwid.signals import connect_signal



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
		cur = self.body.get_focus()
		if cur[1] >= len(self.body)-1:
			return
			
		nextRow = self.body.get_next(cur[1])
		self.body.set_focus(nextRow[1])
			
	def focusPrevious(self):
		cur = self.body.get_focus()
		if cur[1] == 0:
			return
			
		self.body.set_focus(self.body.get_prev(cur[1])[1])

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
	def terminal2markup(ss, invert=0):
		#source = "\033[31mFOO\033[0mBAR"
		table = {"[1":("bold",'bold_f'), "[31":('redfg','redfg_f'), "[32":('greenfg', "greenfg_f"), 
			"[33":('yellowfg', "yellowfg_f"), "[36":('cyanfg', "cyanfg_f"), "[41":("redbg", "regbg_f"), "[0":('std', "std_f"), "[":('std', "std_f")}
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
				markup.append((table[attr][invert], text))
			
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
			line2 = Urwid.terminal2markup(line)
			btn = mButton(line2, onClick)
			btn.origText = line
			btn = urwid.AttrMap(btn, None, "reveal focus")
			outList.append(btn)
		return outList
		
	def popupMsg(title, ss):
		def onCloseBtn(btn):
			g.mainLoop.widget = g.mainLoop.widget.bottom_w
			
		txtMsg = urwid.Text(ss)
		btnClose = urwid.Button("Close", onCloseBtn)
		popup = urwid.LineBox(urwid.Pile([('pack', txtMsg), ('pack', btnClose)]), title)
		g.mainLoop.widget = urwid.Overlay(urwid.Filler(popup), g.mainLoop.widget, 'center', 20, 'middle', 10)
		
	def popupAsk(title, ss, onOk, onCancel = None):
		def onClickBtn(btn):
			if btn == btnYes:
				onOk()
			elif btn == btnNo:
				if onCancel != None: 
					onCancel()
					
			g.mainLoop.widget = g.mainLoop.widget.bottom_w
			
		txtMsg = urwid.Text(ss)
		btnYes = urwid.Button("Yes", onClickBtn)
		btnNo = urwid.Button("No", onClickBtn)
		popup = urwid.LineBox(urwid.Pile([('pack', txtMsg), ('pack', urwid.Columns([btnYes, btnNo]))]), title)
		g.mainLoop.widget = urwid.Overlay(urwid.Filler(popup), g.mainLoop.widget, 'center', 40, 'middle', 5)
		

def unhandled(key):
	if key == 'f8' or key == "q":
		raise urwid.ExitMainLoop()
	elif key == 'k':
		g.widgetContent.scrollUp()
	elif key == 'j':
		g.widgetContent.scrollDown()
	elif key == "[":
		g.widgetFileList.focusPrevious()
		onFileSelected(g.widgetFileList.focus)
	elif key == "]":
		g.widgetFileList.focusNext()
		onFileSelected(g.widgetFileList.focus)
	elif key == "a":
		def onAdd():
			system("git add %s" % fname)
			refreshFileList()
				
		btn = g.widgetFileList.focus
		fname = getFileNameFromBtn(btn)
		Urwid.popupAsk("Git add", "Do you want to add a file[%s]?" % fname, onAdd)

	elif key == "r":
		def onReset():
			system("git reset %s" % fname)
			refreshFileList()
				
		btn = g.widgetFileList.focus
		fname = getFileNameFromBtn(btn)
		Urwid.popupAsk("Git reset", "Do you want to reset a file[%s]?" % fname, onReset)
		
	elif key == "h":
		Urwid.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")
		
def inputFilter(keys, raw):
	return keys

def getFileNameFromBtn(btn):
	label = btn.base_widget.get_label()
	return label[2:].strip()
		

def onFileSelected(btn):
	# why btn.get_label() is impossible?
	label = btn.base_widget.get_label()
	g.selectFileName = getFileNameFromBtn(btn)

	#g.headerText.set_text("file - " + label)
	
	# display
	if label.startswith("?? "):
		try:
			ss = open(g.selectFileName, "r", encoding="UTF-8").read()
		except UnicodeDecodeError:
			Urwid.popupMsg("Encoding", "Encoding error[%s]" % g.selectFileName);
			ss = "Error to load"
			
	else:
		ss = system("git diff --color %s" % g.selectFileName)
		
	ss = ss.replace("\t", "    ")
		
	del g.widgetContent.body[:]
	g.widgetContent.body += Urwid.makeTextList(ss.split("\n"))
	g.widgetFrame.set_focus(g.widgetContent)

	
def refreshFileList():
	lstFile = system("git -c color.status=always status -s")
	del g.widgetFileList.body[:]
	g.widgetFileList.body += Urwid.makeBtnList(lstFile.split("\n"), onFileSelected)

def urwidGitStatus():
	lstFile = ""
	lstContent = ["test"]
	
	def onFileFocusChanged(new_focus):
		# old widget
		widget = g.widgetFileList.focus
		widget.base_widget._label.set_text(Urwid.terminal2markup(widget.base_widget.origText, 0))

		widget = g.widgetFileList.body[new_focus]
		widget.base_widget._label.set_text(Urwid.terminal2markup(widget.base_widget.origText, 1))

	g.widgetFileList = mListBox(urwid.SimpleFocusListWalker(Urwid.makeBtnList(lstFile.split("\n"), onFileSelected)))
	g.widgetFileList.body.set_focus_changed_callback(onFileFocusChanged)
	g.widgetContent = mListBox(urwid.SimpleListWalker(Urwid.makeTextList(lstContent)))
	g.widgetFrame = urwid.Pile([(8, urwid.AttrMap(g.widgetFileList, 'std')), ('pack', urwid.Divider('-')), g.widgetContent])
	
	g.headerText = urwid.Text(">> dc V1.0 - Q(Quit), A(Add), R(Reset), C(Commit), I(Ignore), [/](Prev/Next file)")
	g.mainWidget = urwid.Frame(g.widgetFrame, header=g.headerText)
		
	refreshFileList()
	
	# (name, fg, bg, mono, fgHigh, bgHigh)
	palette = [
		('std', 'light gray', 'black'),
		('std_f', 'black', 'dark cyan'),
		('reset', 'std'),
		("reset_f", "std_f"),
		('bold', 'light gray,bold', 'black'),
		('bold_f', 'light gray,bold', 'dark cyan'),

		('redfg', 'dark red', 'black'),
		('redfg_f', 'light red', 'dark cyan'),
		('greenfg', 'dark green', 'black'),
		('greenfg_f', 'light green', 'dark cyan'),
		('yellowfg', 'yellow', 'black'),
		('yellowfg_f', 'yellow', 'dark cyan'),
		('bluefg', 'dark blue', 'black'),
		('bluefg_f', 'light blue', 'dark cyan'),
		('cyanfg', 'dark cyan', 'black'),
		('cyanfg_f', 'light gray', 'dark cyan'),
		
		('redbg', 'black', 'dark red'),
		
		('reveal focus', "black", "dark cyan", "standout"),
		
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

	g.mainLoop = urwid.MainLoop(g.mainWidget, palette, screen,
		unhandled_input=unhandled, input_filter=inputFilter)
	g.mainLoop.run()
		
		

g = Global()
g.log = open("log.log", "w", encoding="UTF-8")
g.selectFileName = ""	#

g.mainLoop = None	# urwid

g.mainWidget = None
g.widgetFrame = None
g.headerText = "" 
g.widgetFileList = None
g.widgetContent = None


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
		ret = run()
	except ExcFail as e:
		print(e)
		sys.exit(1)
	

