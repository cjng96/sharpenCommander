# coding: utf-8
#!/usr/bin/env python3

import subprocess

import os
import sys
import select
import datetime
import re
import stat

from enum import Enum


import urwid
import urwid.raw_display
import urwid.web_display
from urwid.signals import connect_signal


import tool
from tool import git, system, systemSafe, systemRet, programPath

from globalBase import *

import urwidHelper as ur



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


Color = Enum('color', 'blue red')

class Ansi:
	redBold = "\033[1;31m"
	red = "\033[0;31m"
	blueBold = "\033[1;34m"
	blue = "\033[0;34m"
	clear = "\033[0m"


class MyProgram(Program):
	def __init__(self):
		super().__init__("1.1.0", programPath("dc.log"))
		self.lstPath = []
		self.configPath = ""    # ~/.devcmd/path.py
		self.isPrintSystem = False


	def init(self):
		pp = os.path.expanduser("~/.devcmd")
		if not os.path.isdir(pp):
			print("No .devcmd folder. generate it...")
			os.mkdir(pp)
			
		self.configPath = os.path.join(pp, "path.py")	
		
		if not os.path.isfile(g.configPath):
			raise ErrFailure("No path.py file in ~/.devcmd")

		sys.path.append(pp)
		m = __import__("path")
		self.lstPath = [ item for item in m.pathList if len(item["name"]) > 0 ]
		
		for item in g.lstPath:
			item["path"] = os.path.expanduser(item["path"])
			name = item["name"]
			if type(name) is str:
				item["name"] = [name]
		
	def savePath(self, pp):
		with open("/tmp/cmdDevTool.path", "wb") as f:
			f.write(os.path.expanduser(pp).encode())
			
	def findItem(self, target):
		for pp in self.lstPath:
			lstName = pp["name"]

			if target.lower() in map(str.lower, lstName):
				return pp
				
		raise ErrFailure("No that target[%s]" % target)
		
	def cd(self, target):
		if target == "~":
			self.savePath(target)
			return
	
		item = self.findItem(target)
		self.savePath(item["path"])

	def listPath(self):
		for pp in self.lstPath:
			print(pp)

	def printCommitLogForPush(self, currentBranch, remoteBranch):
		# commit log to push
		gap = git.commitGap(currentBranch, remoteBranch)
		if gap == 0:
			git.printStatus()
			raise ErrFailure("There is no commit to push")

		print("There are %d commits to push" % gap)
		ss = git.commitLogBetween(currentBranch, remoteBranch)
		print(ss)

	def gitPush(self):
		currentBranch = git.getCurrentBranch()
		remoteBranch = git.getTrackingBranch()
		if remoteBranch is None:
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
							ss,st = git.rebase(remoteBranch)
							# exe result?
							print(ss)
							if st != 0:
								raise Exception("rebase failed. you should do [rebase --abort][%d]" % st)
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
			raise ErrFailure("Push is canceled")
			

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




def refreshBtnList(content, listBox, onClick):
	del listBox.body[:]
	if content.strip() == "":
		contentList = ["< Nothing >"]
		listBox.itemCount = 0
	else:
		contentList = content.split("\n")
		listBox.itemCount = len(contentList)
		
	listBox.body += ur.makeBtnList(contentList, onClick)


class AckFile:
	def __init__(self, fnameTerminal):
		self.fname = ur.termianl2plainText(fnameTerminal)
		#self.fnameMarkup = Urwid.terminal2markup(fnameTerminal, 0)
		#self.fnameOrig = fnameTerminal

		self.lstLine = []	
		
	def getTitleMarkup(self, focus=False):
		themeTitle = "greenfg" if not focus else "greenfg_f"
		themeCount = "std" if not focus else "std_f"  
		return [(themeTitle, self.fname), (themeCount, "(%d)" % len(self.lstLine))]


class mDlgMainAck(ur.cDialog):
	def __init__(self):
		super().__init__()

		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnList([], None)))
		self.widgetFileList.body.set_focus_changed_callback(lambda new_focus: self.onFileFocusChanged(new_focus))
		self.widgetContent = ur.mListBox(urwid.SimpleListWalker(ur.makeTextList([])))

		self.header = ">> dc V%s - ack-grep - q/F4(Quit),<-/->(Prev/Next file),Enter(goto),E(edit)..." % g.version
		self.headerText = urwid.Text(self.header)
		self.widgetFrame = urwid.Pile([(15, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)
		
		self.cbFileSelect = lambda btn: self.onFileSelected(btn)
		self.buf = ""
		self.lstContent = []
		
	def btnUpdate(self, btn, focus):
		btn.base_widget.set_label(btn.afile.getTitleMarkup(focus))
		return btn

	def onFileFocusChanged(self, new_focus):
		self.btnUpdate(self.widgetFileList.focus, False)
		newBtn = self.btnUpdate(self.widgetFileList.body[new_focus], True)
		
		self.widgetContent.focus_position = newBtn.afile.position

	def onFileSelected(self, btn):
		pp = os.path.dirname(os.path.join(os.getcwd(), btn.afile.fname))
		g.savePath(pp)
		raise urwid.ExitMainLoop()
		
	def inputFilter(self, keys, raw):
		if g.loop.widget != g.dialog.mainWidget:
			return keys
			
		if ur.filterKey(keys, "down"):
			self.widgetContent.scrollDown()

		if ur.filterKey(keys, "up"):
			self.widgetContent.scrollUp()

		if ur.filterKey(keys, "enter"):
			self.onFileSelected(self.widgetFileList.focus)

		return keys
		
	def recvData(self, data):
		ss = data.decode("UTF-8", "ignore")
		self.buf += ss
		pt = self.buf.rfind("\n")
		if pt == -1:
			return True

		ss = self.buf[:pt]
		self.buf = self.buf[pt:]
		
		for line in ss.splitlines():
			line = line.strip()
			
			if line != "" and ":" not in line:	# file name
				# new file				
				afile = AckFile(line)
				self.lstContent.append(afile)
				
				btn = ur.genBtnMarkup(afile.getTitleMarkup(False), self.cbFileSelect, len(self.widgetFileList.body) == 0)
				btn.afile = afile
				afile.btn = btn
				afile.position = len(self.widgetContent.body)
				self.widgetFileList.body.append(btn)
				
				txt = urwid.Text(afile.getTitleMarkup(False))
				self.widgetContent.body.append(txt)
				
			else:
				afile = self.lstContent[len(self.lstContent)-1]
				line = line.replace("\t", "    ")
				afile.lstLine.append(line)
				
				# update content
				txt = ur.genText(line)
				self.widgetContent.body.append(txt)
				
				self.btnUpdate(afile.btn, afile.position == 0)
			
		return True
			

	def unhandled(self, key):
		if key == 'f4' or key == "q":
			raise urwid.ExitMainLoop()
		elif key == 'left' or key == "[":
			self.widgetFileList.focusPrevious()
		elif key == 'right' or key == "]":
			self.widgetFileList.focusNext()

		elif key == "k":
			self.widgetContent.scrollUp()

		elif key == "j":
			self.widgetContent.scrollDown()

		elif key == "e" or key == "E":
			btn = self.widgetFileList.focus
			g.loop.stop()
			systemRet("vim %s" % btn.afile.fname)
			g.loop.start()
			
		elif key == "h":
			ur.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")
	

class mDlgMainFind(ur.cDialog):
	def __init__(self):
		super().__init__()

		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnList([], None)))
		self.widgetFileList.body.set_focus_changed_callback(lambda new_focus: self.onFileFocusChanged(new_focus))
		self.widgetContent = ur.mListBox(urwid.SimpleListWalker(ur.makeTextList(["< Nothing to display >"])))
		self.widgetContent.isViewContent = True

		self.header = ">> dc V%s - find - q/F4(Quit),<-/->(Prev/Next file),Enter(goto),E(edit)..." % g.version
		self.headerText = urwid.Text(self.header)
		self.widgetFrame = urwid.Pile([(15, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)
		
		self.cbFileSelect = lambda btn: self.onFileSelected(btn)
		self.content = ""
		self.selectFileName = ""

	def onFileFocusChanged(self, new_focus):
		# old widget
		widget = self.widgetFileList.focus
		markup = ("std", widget.base_widget.origText)
		widget.base_widget.set_label(markup)

		widget = self.widgetFileList.body[new_focus]
		markup = ("std_f", widget.base_widget.origText)
		widget.base_widget.set_label(markup)

		self.widgetFileList.set_focus_valign("middle")

		self.selectFileName = gitFileBtnName(widget)

		try:
			with open(self.selectFileName, "r", encoding="UTF-8") as fp:
				ss = fp.read()
		except UnicodeDecodeError:
			ss = "No utf8 file[size:%d]" % os.path.getsize(self.selectFileName) 
			
		ss = ss.replace("\t", "    ")
			
		del self.widgetContent.body[:]
		self.widgetContent.body += ur.makeTextList(ss.splitlines())
		self.widgetFrame.set_focus(self.widgetContent)

	def onFileSelected(self, btn):
		self.selectFileName = gitFileBtnName(btn)
		pp = os.path.dirname(os.path.join(os.getcwd(), self.selectFileName))
		g.savePath(pp)
		raise urwid.ExitMainLoop()
		
	def inputFilter(self, keys, raw):
		if ur.filterKey(keys, "down"):
			self.widgetContent.scrollDown()

		if ur.filterKey(keys, "up"):
			self.widgetContent.scrollUp()

		if ur.filterKey(keys, "enter"):
			self.onFileSelected(self.widgetFileList.focus)

		return keys
		
	def recvData(self, data):
		ss = data.decode("UTF-8")
		self.content += ss
		pt = self.content.rfind("\n")
		if pt == -1:
			return True

		ss = self.content[:pt]
		self.content = self.content[pt:]
		
		for line in ss.splitlines():
			line = line.strip()
			if line == "":
				continue
			
			btn = ur.genBtn(line, self.cbFileSelect, len(self.widgetFileList.body) == 0)
			self.widgetFileList.body.append(btn)
			if len(self.widgetFileList.body) == 1:
				self.onFileFocusChanged(0)
			
		return True

	def unhandled(self, key):
		if key == 'f4' or key == "q":
			raise urwid.ExitMainLoop()
		elif key == 'left' or key == "[":
			self.widgetFileList.focusPrevious()
		elif key == 'right' or key == "]":
			self.widgetFileList.focusNext()

		elif key == "k":
			self.widgetContent.scrollUp()
		elif key == "j":
			self.widgetContent.scrollDown()
		
		elif key == "e" or key == "E":
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)

			g.loop.stop()
			systemRet("vim %s" % fname)
			g.loop.start()
			
		elif key == "h":
			ur.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")

class mDlgMainDc(ur.cDialog):
	def __init__(self):
		super().__init__()

		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnList(["< No files >"], None)))
		self.widgetFileList.body.set_focus_changed_callback(lambda newFocus: self.onFileFocusChanged(newFocus))
		self.widgetExtraList = ur.mListBox(urwid.SimpleListWalker(ur.makeTextList(["< Nothing to display >"])))

		self.title = ">> dc V%s" % g.version
		self.headerText = urwid.Text(self.title)
		self.widgetFrame = urwid.Columns([(120, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetExtraList])
		self.edInput = ur.genEdit("Input commit message => ", "", lambda edit,text: self.onMsgChanged(edit,text))
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText, footer=self.edInput)

	def init(self):
		self.fileRefresh()
		return True

	def onFileFocusChanged(self, newFocus):
		# old widget
		widget = self.widgetFileList.focus
		markup = ur.terminal2markup(widget.base_widget.origText, 0)
		widget.base_widget.set_label(markup)

		widget = self.widgetFileList.body[newFocus]
		markup = ur.terminal2markup(widget.base_widget.origText, 1)
		widget.base_widget.set_label(markup)

	def onMsgChanged(self, edit, text):
		pass

	def fileRefresh(self):
		pp = os.getcwd()
		self.headerText.set_text("%s - %s" % (self.title, os.getcwd()))

		lst = [os.path.join(pp, x) for x in os.listdir(pp)]
		lst2 = [ (x, os.stat(x)) for x in lst]
		lst2.sort(key=lambda s1: -11 if stat.S_ISDIR(s1[1].st_mode) else 1)

		#refreshBtnList(fileList2, self.widgetFileList, lambda btn: self.onFileSelected(btn))
		del self.widgetFileList.body[:]
		self.widgetFileList.itemCount = len(lst2)
		self.widgetFileList.body += ur.makeBtnList( [os.path.basename(x[0]) for x in lst2], None)

	def inputFilter(self, keys, raw):
		if g.loop.widget != g.dialog.mainWidget:
			return keys

		if ur.filterKey(keys, "enter"):
			pp = self.getFocusPath()
			if os.path.isdir(pp):
				os.chdir(os.path.join(pp))
				self.fileRefresh()

		"""
		if ur.filterKey(keys, "left"):
			pp = os.getcwd()
			pp = os.path.dirname(pp)
			os.chdir(pp)
			self.fileRefresh()
		"""

		"""
		if "down" in keys:
			self.widgetContent.scrollDown()
			return self.excludeKey(keys, "down")
		"""

		return keys

	def getFocusPath(self):
		pp = os.getcwd()
		btn = self.widgetFileList.focus
		fname = btn.base_widget.get_label()
		return os.path.join(pp, fname)

	def unhandled(self, key):
		print(key)
		if key == 'f4':
			raise urwid.ExitMainLoop()
		elif key == "meta left":
			pp = os.getcwd()
			pp = os.path.dirname(pp)
			os.chdir(pp)
			self.fileRefresh()

		elif key == "alt e":
			pp = self.getFocusPath()

			g.loop.stop()
			systemRet("vim %s" % pp)
			g.loop.start()
			self.fileRefresh()

		elif key == "h":
			ur.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")


class mDlgMainGitStatus(ur.cDialog):
	def __init__(self):
		super().__init__()

		self.selectFileName = ""

		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnList(["< No files >"], None)))
		self.widgetFileList.body.set_focus_changed_callback(lambda newFocus: self.onFileFocusChanged(newFocus))
		self.widgetContent = ur.mListBox(urwid.SimpleListWalker(ur.makeTextList(["< Nothing to display >"])))

		self.headerText = urwid.Text(">> dc V%s - q/F4(Quit),<-/->(Prev/Next file),A(Add),P(Prompt),R(Reset),D(drop),C(Commit),I(Ignore)" % g.version)
		self.widgetFrame = urwid.Pile([(8, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)

		try:
			g.gitRoot = system("git rev-parse --show-toplevel")
		except subprocess.CalledProcessError:
			print("Current folder is no git repo")
			raise urwid.ExitMainLoop
			
		g.curPath = os.getcwd()
		g.relRoot = "./"
		if g.gitRoot != g.curPath:
			g.relRoot = os.path.relpath(g.gitRoot, g.curPath)

	def init(self):
		self.refreshFileList()
		if self.widgetFileList.itemCount == 0:
			print("No modified or untracked files")
			return False

		return True

	def onFileFocusChanged(self, new_focus):
		# old widget
		widget = self.widgetFileList.focus
		markup = ur.terminal2markup(widget.base_widget.origText, 0)
		widget.base_widget.set_label(markup)

		widget = self.widgetFileList.body[new_focus]
		markup = ur.terminal2markup(widget.base_widget.origText, 1)
		widget.base_widget.set_label(markup)

	def onFileSelected(self, btn):
		# why btn.get_label() is impossible?
		label = btn.base_widget.get_label()
		#self.selectFileName = gitFileBtnName(btn)
		self.selectFileName = gitFileLastName(btn)
		#g.headerText.set_text("file - " + label)
		
		# display
		if label == "< Nothing >":
			ss = label
		elif label.startswith("?? "):
			if os.path.isdir(self.selectFileName):
				ss = "%s is folder" % self.selectFileName
			else:
				try:
					with open(self.selectFileName, "r", encoding="UTF-8") as fp:
						ss = fp.read()
				except UnicodeDecodeError:
					#ur.popupMsg("Encoding", "Encoding error[%s]" % self.selectFileName);
					ss = "No utf8 file[size:%d]" % os.path.getsize(self.selectFileName)
				
		else:
			try:
				ss = system("git diff --color \"%s\"" % self.selectFileName)
			except subprocess.CalledProcessError as e:
				ss = "failed to print diff for %s\n  %s" % (self.selectFileName, e)
			
		ss = ss.replace("\t", "    ")
			
		del self.widgetContent.body[:]
		self.widgetContent.body += ur.makeTextList(ss.splitlines())
		self.widgetFrame.set_focus(self.widgetContent)

	def refreshFileContentCur(self):
		self.onFileSelected(self.widgetFileList.focus)

	def refreshFileList(self, focusMove=0):
		fileList = system("git -c color.status=always status -s")
		
		# quoted octal notation to utf8
		fileList = bytes(fileList, "utf-8").decode("unicode_escape")
		bb = fileList.encode("ISO-8859-1")
		fileList = bb.decode()
		
		# remove "" in file name
		fileList2 = ""
		for line in fileList.splitlines():
			fileType, fileName = line.split(" ", 1)
			if fileName.startswith("\"") and fileName.endswith("\""):
				fileName = fileName[1:-1]  
			fileList2 += fileType + " " + fileName + "\n"
		
		focusIdx = self.widgetFileList.focus_position + focusMove
		refreshBtnList(fileList2, self.widgetFileList, lambda btn: self.onFileSelected(btn))
		if focusIdx >= len(self.widgetFileList.body):
			focusIdx = len(self.widgetFileList.body)-1
		self.widgetFileList.focus_position = focusIdx
	
		self.onFileSelected(self.widgetFileList.focus)	# auto display
		
	def gitGetStagedCount(self):
		cnt = 0
		for item in self.widgetFileList.body:
			ss = item.base_widget.origText
			if "[32m" in ss:	# greenfg
				cnt += 1
		return cnt

	def inputFilter(self, keys, raw):
		if g.loop.widget != g.dialog.mainWidget:
			return keys

		if ur.filterKey(keys, "down"):
			self.widgetContent.scrollDown()

		if ur.filterKey(keys, "up" ):
			self.widgetContent.scrollUp()

		return keys

	def unhandled(self, key):
		if key == 'f4' or key == "q":
			raise urwid.ExitMainLoop()
		elif key == 'k':
			self.widgetContent.scrollUp()
		elif key == 'j':
			self.widgetContent.scrollDown()
		elif key == "left" or key == "[" or key == "f11":
			self.widgetFileList.focusPrevious()
			self.refreshFileContentCur()
		elif key == "right" or key == "]" or key == "f12":
			self.widgetFileList.focusNext()
			self.refreshFileContentCur()
			
		elif key == "A":
			btn = self.widgetFileList.focus
			#fname = gitFileBtnName(btn)
			fname = gitFileLastName(btn)
			system("git add \"%s\"" % fname)
			self.refreshFileList(1)
			
		elif key == "P":
			def onPrompt():
				g.loop.stop()
				systemRet("git add -p \"%s\"" % fname)
				g.loop.start()
				self.refreshFileList()
					
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			ur.popupAsk("Git add", "Do you want to add a file via prompt[%s]?" % fname, onPrompt)

		elif key == "R":
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			system("git reset \"%s\"" % fname)
			self.refreshFileList()
			
		elif key == "D":
			def onDrop():
				system("git checkout -- \"%s\"" % fname)
				self.refreshFileList()
					
			def onDelete():
				os.remove(fname)
				self.refreshFileList()
					
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			if gitFileBtnType(btn) == "??":
				ur.popupAsk("Git reset(f)", "Do you want to delete file[%s]?" % fname, onDelete)
			else:
				ur.popupAsk("Git reset(f)", "Do you want to drop file[%s]s modification?" % fname, onDrop)
		
		elif key == "E":
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)

			g.loop.stop()
			systemRet("vim %s" % fname)
			g.loop.start()
			
			self.refreshFileContentCur()
			
		elif key == "c_old":
			def onCommit():
				g.loop.stop()
				systemRet("git commit")
				g.loop.start()
				self.refreshFileList()
					
			ur.popupAsk("Git commit", "Do you want to commit?", onCommit)

		elif key == "C":
			def onExit():
				g.dialog = self
				g.loop.widget = self.mainWidget
				self.refreshFileList()
				
				# exit
				if self.widgetFileList.itemCount == 0:
					g.loop.stop()
					print("No modified or untracked files")
					sys.exit(0)
					
			# check staged data 
			n = self.gitGetStagedCount()
			if n == 0:
				ur.popupMsg("Alert", "There is no staged file to commit")
				return
				
			dlg = mGitCommitDialog(onExit)
			g.dialog = dlg
			g.loop.widget = dlg.mainWidget
			
		elif key == "h":
			ur.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")


class mGitCommitDialog(ur.cDialog):
	themes = [("greenfg", "greenfg_f"), ("std", "std_f")]
	
	def __init__(self, onExit):
		super().__init__()

		self.selectFileName = ""

		self.onExit = onExit
		self.edInput = ur.genEdit("Input commit message => ", "", lambda edit,text: self.onMsgChanged(edit,text))
		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnList(["< No files >"], None)))
		self.widgetFileList.body.set_focus_changed_callback(lambda new_focus: self.onFileFocusChanged(new_focus))
		self.widgetContent = ur.mListBox(urwid.SimpleListWalker(ur.makeTextList(["< Nothing to display >"])))

		self.headerText = urwid.Text(">> Commit...")
		self.widgetFrame = urwid.Pile([("pack", self.edInput), (8, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)
		
		self.refreshFileList()
		self.widgetFrame.set_focus(self.edInput)

	def onMsgChanged(self, edit, text):
		pass
		
	def _applyFileColorTheme(self, widget, isFocus=0):
		theme = self.themes[0 if widget.base_widget.data == "s" else 1]
		widget.base_widget.set_label((theme[isFocus], widget.base_widget.origText))
	

	def onFileFocusChanged(self, new_focus):
		# old widget
		widget = self.widgetFileList.focus
		self._applyFileColorTheme(widget, 0)

		widget = self.widgetFileList.body[new_focus]
		self._applyFileColorTheme(widget, 1)

	def onFileSelected(self, btn):
		# why btn.get_label() is impossible?
		label = btn.base_widget.get_label()
		self.selectFileName = btn.base_widget.get_label()
		#g.headerText.set_text("file - " + label)
		
		# display
		btnType = btn.base_widget.data
		pp = os.path.join(g.relRoot, self.selectFileName)
		try:
			ss = system("git diff --color %s \"%s\"" % ("" if btnType == "c" else "--staged", pp))
		except subprocess.CalledProcessError as e:
			ss = "failed to print diff for %s\n  %s" % (pp, e)
			
		ss = ss.replace("\t", "    ")
			
		del self.widgetContent.body[:]
		self.widgetContent.body += ur.makeTextList(ss.split("\n"))
		self.widgetFrame.set_focus(self.widgetContent)

	def refreshFileContentCur(self):
		self.onFileSelected(self.widgetFileList.focus)

	def refreshFileList(self):
		del self.widgetFileList.body[:]

		# staged file list		
		fileList = system("git diff --name-only --cached")
		self.widgetFileList.body += ur.makeBtnList(fileList.split("\n"),
			lambda btn: self.onFileSelected(btn), 
			lambda btn: setattr(btn, "data", "s"))

		# general file list
		fileList = system("git diff --name-only")
		self.widgetFileList.body += ur.makeBtnList(fileList.split("\n"),
			lambda btn: self.onFileSelected(btn), 
			lambda btn: setattr(btn, "data", "c"))
			
		for widget in self.widgetFileList.body:
			self._applyFileColorTheme(widget, 0)
			
		if len(self.widgetFileList.body) == 0:
			self.widgetFileList.body += ur.makeBtnList(["< Nothing >"], None)
		else:
			self.onFileFocusChanged(self.widgetFileList.focus_position)
			self.onFileSelected(self.widgetFileList.focus)	# auto display

	def inputFilter(self, keys, raw):
		if g.loop.widget != g.dialog.mainWidget:
			return keys

		if ur.filterKey(keys, "down"):
			self.widgetContent.scrollDown()

		if ur.filterKey(keys, "up"):
			self.widgetContent.scrollUp()

		return keys
		
	def unhandled(self, key):
		if key == "q" or key == "Q" or key == "f4":
			self.onExit()
		elif key == 'k':
			self.widgetContent.scrollUp()
		elif key == 'j':
			self.widgetContent.scrollDown()
		elif key == "left" or key == "[" or key == "f11":
			self.widgetFileList.focusPrevious()
			self.refreshFileContentCur()

			if key == "f11":
				self.widgetFrame.set_focus(self.edInput)

		elif key == "right" or key == "]" or key == "f12":
			self.widgetFileList.focusNext()
			self.refreshFileContentCur()

			if key == "f12":
				self.widgetFrame.set_focus(self.edInput)

		elif key == "A":
			def onAdd():
				system("git add \"%s\"" % fname)
				self.refreshFileList()
					
			def onPrompt():
				g.loop.stop()
				systemRet("git add -p \"%s\"" % fname)
				g.loop.start()
				self.refreshFileList()
					
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			ur.popupAsk3("Git add", "Do you want to add a file[%s]?" % fname, "Add", "Prompt", "Cancel", onAdd, onPrompt)

		elif key == "R":
			def onReset():
				system("git reset \"%s\"" % fname)
				self.refreshFileList()
					
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			ur.popupAsk("Git reset", "Do you want to reset a file[%s]?" % fname, onReset)
			
		elif key == "D":
			def onDrop():
				system("git checkout -- \"%s\"" % fname)
				self.refreshFileList()
					
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			ur.popupAsk("Git reset(f)", "Do you want to drop file[%s]s modification?" % fname, onDrop)
		
		elif key == "E":
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)

			g.loop.stop()
			systemRet("vim %s" % fname)
			g.loop.start()
			
			self.refreshFileContentCur()
			
		elif key == "esc":
			self.widgetFrame.set_focus(self.edInput)
			
		elif key == "ctrl a":
			# commit all
			def onCommit():
				tt = self.edInput.get_edit_text()
				ss = system("git commit -a -m \"%s\"" % tt[:-1])
				self.onExit()
					
			ur.popupAsk("Git Commit", "Do you want to commit all modification?", onCommit)
			
		elif key == "enter":
			# commit
			tt = self.edInput.get_edit_text()
			ss = system("git commit -m \"%s\"" % tt)
			#print(ss)
			self.onExit()

		elif key == "C":
			def onCommit():
				g.loop.stop()
				systemRet("git commit -a")
				g.loop.start()
				self.refreshFileList()
					
			ur.popupAsk("Git commit(all)", "Do you want to commit all content?", onCommit)
			
		elif key == "h":
			ur.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")



def urwidUnhandled(key):
	g.dialog.unhandled(key)
		
def urwidInputFilter(keys, raw):
	op = getattr(g.dialog, "inputFilter", None)
	if not callable(op):
		return keys
		
	return g.dialog.inputFilter(keys, raw)

def gitFileBtnName(btn):
	label = btn.base_widget.get_label()
	return label[2:].strip()

# "??" - untracked file
def gitFileBtnType(btn):
	label = btn.base_widget.get_label()
	return label[:2]

def unwrapQutesFilename(ss):
	if ss.startswith('"'):
		# escape including qutes
		ss = ss[1:-1].replace('"', '\\"')
		return ss
	else:
		return ss

def gitFileLastName(btn):
	ftype = gitFileBtnType(btn)
	fname = gitFileBtnName(btn)
	#R  b -> d
	#R  "test a.txt" -> "t sp"
	#A  "test b.txt"
	#A  "tt \"k\" tt"
	if not ftype.startswith("R"):
		return unwrapQutesFilename(fname)

	# case1. a -> b
	if not fname.startswith("\""):
		pt = fname.rindex(" -> ")
		fname = fname.substring(pt)
		return unwrapQutesFilename(fname)
	else:
		# case2. "test a" -> "test b"
		ss = fname[:-1]
		while True:
			pt = ss.rfind('"')
			if pt == 0:
				return ss[1:]

			if pt != -1:
				if ss[pt-1] != "\\":
					return ss[pt+1:]
				else:
					# TODO:
					raise Exception("Not supported file format[%s]" % fname)


from distutils.spawn import find_executable

def uiMain(dlgClass, doSubMake=None):
	try:
		dlg = dlgClass()
	except urwid.ExitMainLoop:
		return

	if not dlg.init():
		return

	g.dialog = dlg
	g.loop = urwid.MainLoop(dlg.mainWidget, g.palette, urwid.raw_display.Screen(),
							unhandled_input=urwidUnhandled, input_filter=urwidInputFilter)

	if doSubMake is not None:
		writeFd = g.loop.watch_pipe(lambda data: dlg.recvData(data))
		g.subProc = doSubMake(writeFd)

		def subCheck(_handle, _userData):
			if g.subProc.poll() is not None:
				dlg.headerText.set_text(dlg.header + "!!!")
				#g.loop.remove_alarm(handle)
			else:
				g.subTimerHandler = g.loop.set_alarm_in(0.1, subCheck, None)

		subCheck(None, None)

	g.loop.run()

# workItemIdx: 지정되면 해당 번째 다음께 target이 된다.
def doSubCmd(cmds, dlgClass, targetItemIdx=-1):
	cmds[0] = find_executable(cmds[0])
	
	if targetItemIdx != -1 and len(sys.argv) == targetItemIdx:
		target = cmds[targetItemIdx]
		item = g.findItem(target)
		os.chdir(item["path"])
		cmds = cmds[:targetItemIdx] + cmds[targetItemIdx+1:]

	uiMain(dlgClass, lambda writeFd: subprocess.Popen(cmds, bufsize=0, stdout=writeFd, close_fds=True))


class Gr(object):
	def __init__(self):
		self.isInit = False
		self.repoList = [dict(name=["test"], path="")]
		
	def init(self):
		self.repoList = [repo for repo in g.lstPath if "repo" in repo and repo["repo"] != 0]
		self.isInit = True
		
	def repoAllName(self):
		return [repo["name"][0] for repo in self.repoList]
		
	def action(self, action):
		if not self.isInit:
			self.init()

		if len(sys.argv) >= 3:
			second = sys.argv[2]
			if second == ".":
				# current repo
				cur = os.getcwd() + "/"
				for repo in gr.repoList:
					repoPath = os.path.realpath(repo["path"]) 
					if cur.startswith(repoPath+"/"):
						second = repo["name"][0]
						break
				if second == ".":
					self.log(0, "Current path[%s] is not git repo." % cur)
					return
				
			action(self, second)
			
		else:
			for comp in gr.repoAllName():
				action(self, comp)
		
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

	def getRepoPath(self, name):
		repo = self.getRepo(name)
		path = repo["path"]
		return path
				
	def changePath(self, name):
		path = self.getRepoPath(name)
		if not os.path.isdir(path):
			raise FileNotFoundError(path, "%s(%s) -> doesn't exist"  % (name, path))

		os.chdir(path)
		ss = "path:%s" % path
		return ss

				
	def checkSameWith(self, name, branchName, remoteBranch):
		rev = git.rev(branchName)
		rev2 = git.rev("remotes/"+remoteBranch)
		isSame = rev == rev2
		if isSame:
			self.log2(Color.blue, name, "%s is same to %s"  % (branchName, remoteBranch))
			return True
		else:
			commonRev = git.commonParentRev(branchName, remoteBranch)
			#print("common - %s" % commonRev)
			if commonRev != rev2:
				self.log2(Color.red, name, "%s(%s) - origin/master(%s) -->> Different" % (branchName, rev, rev2))
				return False
		
			# 오히려 앞선경우다. True로 친다.
			gap = git.commitGap(branchName, remoteBranch)
			self.log2(Color.red, name, "Your local branch(%s) is forward than %s[%d commits]" % (branchName, remoteBranch, gap))
			
			# print commit log
			#ss = system("git log --oneline --graph --all --decorate --abbrev-commit %s..%s" % (remoteBranch, branchName))
			ss = git.commitLogBetween(branchName, remoteBranch)
			print(ss)
			
			return True

	def stashCheck(self, name):
		uname = "###groupRepo###"
		stashName = git.stashGetNameSafe(uname)
		if stashName is not None:
			self.log2(Color.red, name, "YOU HAVE STASH ITEM. PROCESS IT FIRST")
			return False

		return True


	def statusComponent(self, name):
		try:
			path = self.changePath(name)
		except ErrNoExist as e:
			self.log2(Color.red, name, "%s DOESN'T exist" % e.path)
			return

		if not self.stashCheck(name):
			return

		
		branchName = git.getCurrentBranch()
		remoteBranch = git.getTrackingBranch()
		if remoteBranch is None:
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
		try:
			path = self.changePath(name)
		except ErrNoExist as e:
			self.log2(Color.red, name, "%s DOESN'T exist" % e.path)
			return

		if not self.stashCheck(name):
			return

		branchName = git.getCurrentBranch()
		remoteBranch = git.getTrackingBranch()
		if remoteBranch is None:
			self.log2(Color.red, name, "%s DONT'T HAVE TRACKING branch" % branchName)
			return
		
		isSame = self.checkSameWith(name, branchName, remoteBranch)
		if isSame:
			return
	
		repo = self.getRepo(name)
		if "type" in repo and repo["type"] == "bin":
			self.log2(Color.blue, name, "merge with %s - %s - bin type" % (remoteBranch, path))
		
			uname = "###groupRepo###"	
			ss = system("git stash save -u \"%s\"" % uname)
			print(ss)
			ss = system("git merge %s" % remoteBranch)
			print(ss)
			stashName = git.stashGetNameSafe(uname)
			ss = system("git stash pop %s" % stashName)
			print(ss)
	
		diffList = git.checkFastForward(branchName, remoteBranch)
		if len(diffList) != 0:
			self.log2(Color.red, name, "NOT be able to fast forward - %s" % path)
		else:			
			self.log2(Color.blue, name, "merge with %s - %s" % (remoteBranch, path))
			ss = system("git rebase %s" % remoteBranch)
			print(ss)
            
            
	def fetch(self, name):
		try:
			path = gr.changePath(name)
		except ErrNoExist as e:
			self.log2(Color.red, name, "%s DOESN'T exist" % e.path)
			return

		self.log2(Color.blue, name, "fetch --prune - %s" % path)
		system("git fetch --prune")


gr = Gr()


def winTest():
	ss = system("c:\\cygwin64\\bin\\git.exe diff --color dc.py")

	kk = ur.terminal2markup(ss)
	st = ss.find("\x1b")
	print("%d %x %x %x %x" % (st, ss[0], ss[1], ss[2], ss[3]))
	sys.exit(0)

def getNonblocingInput():
	if select.select([sys.stdin], [], [], 0) == ([sys.stdin], [], []):
		return sys.stdin.read(255)

def removeEmptyArgv():		
	#cmds = shlex.split(cmdLine)
	# find with shell=true not working on cygwin
	for idx,data in reversed(list(enumerate(sys.argv))):
		if data != "":
			sys.argv = sys.argv[:idx+1]
			break


def run():
	#winTest()
	try:
		os.remove("/tmp/cmdDevTool.path")
	except OSError:
		pass

	# under pipe line
	'''
	ss = getNonblocingInput()
	if ss != None:
		ss = ss.strip("\n")
		if ss == "":
			print("Empty path in pipe")
			return
		else:
			#ss = os.path.dirname(ss)
			#print("goto: " + ss)
			#g.savePath(ss)
			pass
		return
	'''
	prog = MyProgram()
	prog.init()

	argc = len(sys.argv)	
	if argc == 1:
		target = ""	# basic cmd
	else:
		target = sys.argv[1]
		

	removeEmptyArgv()

	if target == "":
		uiMain(mDlgMainDc)
		return

	elif target == "push":
		print("fetching first...")
		git.fetch()
		g.gitPush()
		return
		
	elif target == "ci":
		uiMain(mDlgMainGitStatus)
		return
		
	elif target == "list":
		g.listPath()
		return
		
	elif target == "config":
		g.savePath("~/.devcmd")
		return
		
	elif target == "which":
		ss, status = systemSafe(" ".join(['"' + c + '"' for c in sys.argv[1:]]))
		print(ss)
		print("goto which path...")
		g.savePath(os.path.dirname(ss))
		return
	
	elif target == "find":
		# dc find . -name "*.py"
		cmds = sys.argv[1:]
		doSubCmd(cmds, mDlgMainFind)
		return
		
	elif target == "findg":
		pp = sys.argv[2]
		if "*" not in pp:
			pp = "*"+pp+"*"

		cmds = ["find", ".", "-name", pp]
		doSubCmd(cmds, mDlgMainFind, 4)
		return
		
	elif target == "ack":
		# dc ack printf
		cmds = sys.argv[1:]
		cmds.insert(1, "--group")
		cmds.insert(1, "--color")
		doSubCmd(cmds, mDlgMainAck)
		return
		
	elif target == "ackg":
		# dc ack printf
		cmds = ["ack"] + sys.argv[2:]
		cmds.insert(1, "--group")
		cmds.insert(1, "--color")
		doSubCmd(cmds, mDlgMainAck, 4)
		return
		
	elif target == "st":
		gr.action(Gr.statusComponent)
		return
		
	elif target == "fetch":
		gr.action(Gr.fetch)
		return
		
	elif target == "merge":
		gr.action(Gr.mergeSafe)
		return
		
	elif target == "update":
		print("fetch......")
		gr.action(Gr.fetch)

		print("merge......")
		gr.action(Gr.mergeSafe)

		print("status......")
		gr.action(Gr.statusComponent)
		return

	#print("target - %s" % target)
	g.cd(target)
	return 1
	

if __name__ == "__main__":
	try:
		ret = run()
	except ErrFailure as e:
		print(e)
		sys.exit(1)
	

