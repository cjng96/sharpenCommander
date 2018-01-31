# coding: utf-8
#!/usr/bin/env python3

import subprocess

import os
import sys
import select
import datetime
import re
import stat
import json
import traceback
import inspect


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
		self.regList = []
		self.configPath = ""    # ~/.devcmd/path.py
		self.isPrintSystem = False

		# main dialog
		self.dialog = None
		self.loop = None

	def init(self):
		pp = os.path.expanduser("~/.devcmd")
		if not os.path.isdir(pp):
			print("No .devcmd folder. generate it...")
			os.mkdir(pp)
			
		self.configPath = os.path.join(pp, "path.json")

		self.configLoad()

	def configLoad(self):
		if not os.path.isfile(g.configPath):
			print("No path.json file. generate it...")
			self.regList = []
			self.configSave()
			return

		#sys.path.append(pp)
		#m = __import__("path")
		#self.lstPath = [ item for item in m.pathList if len(item["names"]) > 0 ]
		with open(self.configPath, "r") as fp:
			obj = json.load(fp)
			self.regList = obj["path"]

		for item in self.regList:
			item["path"] = os.path.expanduser(item["path"])
			name = item["names"]
			if type(name) is str:
				item["names"] = [name]

			if "groups" not in item:
				item["groups"] = []


	def configSave(self):
		obj = dict()
		obj["path"] = self.regList
		with open(self.configPath, "w") as fp:
			json.dump(obj, fp, separators=(',',':'))

	def savePath(self, pp):
		with open("/tmp/cmdDevTool.path", "wb") as f:
			f.write(os.path.expanduser(pp).encode())

	def regFindByName(self, target):
		for pp in self.regList:
			names = pp["names"]

			if target.lower() in map(str.lower, names):
				return pp

		raise ErrFailure("No that target[%s]" % target)

	def regFindByPath(self, pp):
		return next((x for x in g.regList if x["path"] == pp), None)

	# path list that includes sub string
	def regFindItems(self, sub):
		sub = sub.lower()
		lst = []
		for pp in self.regList:
			if self.matchItem(pp, sub):
				lst.append(pp)

		return lst

	@staticmethod
	def matchItem(item, sub):
		names = item["names"]
		names2 = map(str.lower, names)

		hasList = list(filter(lambda s: sub in s, names2))
		return len(hasList)

	def cd(self, target):
		if target == "~":
			self.savePath(target)
			return
	
		item = self.regFindByName(target)
		self.savePath(item["path"])

	def regListPrint(self):
		for pp in self.regList:
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

		ss2 = remoteBranch.split("/")		

		# push it	
		ss, status = systemSafe("git push %s %s:%s" % (ss2[0], currentBranch, target))
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

	def doSetMain(self, dlg):
		if not dlg.init():
			dlg.close()
			return False

		self.dialog = dlg
		g.loop.widget = dlg.mainWidget
		return True


"""
itemList = list of (terminal, attr)
"""
def refreshBtnListTerminal(terimalItemList, listBox, onClick):
	del listBox.body[:]
	listBox.itemCount = len(terimalItemList)
	if listBox.itemCount == 0:
		terimalItemList = [("< Nothing > ", None)]

	listBox.body += ur.makeBtnListTerminal(terimalItemList, onClick)

"""
itemList = list of (markup,  attr)
"""
def refreshBtnListMarkupTuple(markupItemList, listBox, onClick):
	del listBox.body[:]
	listBox.itemCount = len(markupItemList)
	if listBox.itemCount == 0:
		markupItemList = [("std", "< Nothing > ", "")]

	listBox.body += ur.makeBtnListMarkup(markupItemList, onClick)


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

		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([], None)))
		self.widgetFileList.setFocusCb(lambda newFocus: self.onFileFocusChanged(newFocus))

		self.widgetContent = ur.mListBox(urwid.SimpleListWalker(ur.makeTextList([])))

		self.header = ">> dc V%s - ack-grep - q/F4(Quit),<-/->(Prev/Next file),Enter(goto),E(edit)..." % g.version
		self.headerText = urwid.Text(self.header)
		self.widgetFrame = urwid.Pile([(15, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)
		
		self.cbFileSelect = lambda btn: self.onFileSelected(btn)
		self.buf = ""
		self.lstContent = []
		
	def btnUpdate(self, btn, focus):
		btn.original_widget.set_label(btn.afile.getTitleMarkup(focus))
		return btn

	def onFileFocusChanged(self, new_focus):
		self.btnUpdate(self.widgetFileList.focus, False)
		newBtn = self.btnUpdate(self.widgetFileList.body[new_focus], True)
		
		self.widgetContent.focus_position = newBtn.afile.position
		return False

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

				isFirst = len(self.widgetFileList.body) == 0
				btn = ur.genBtnMarkup(afile.getTitleMarkup(isFirst), self.cbFileSelect)
				btn.afile = afile
				afile.btn = btn
				afile.position = len(self.widgetContent.body)
				self.widgetFileList.body.append(btn)
				
				txt = urwid.Text(afile.getTitleMarkup(isFirst))
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

		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([], None)))
		self.widgetFileList.setFocusCb(lambda newFocus: self.onFileFocusChanged(newFocus))
		self.widgetContent = ur.mListBox(urwid.SimpleListWalker(ur.makeTextList(["< Nothing to display >"])))
		self.widgetContent.isViewContent = True

		self.header = ">> dc V%s - find - q/F4(Quit),<-/->(Prev/Next file),Enter(goto),E(edit)..." % g.version
		self.headerText = urwid.Text(self.header)
		self.widgetFrame = urwid.Pile([(15, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)
		
		self.cbFileSelect = lambda btn: self.onFileSelected(btn)
		self.content = ""
		self.selectFileName = ""

	def onFileFocusChanged(self, newFocus):
		# old widget
		#widget = self.widgetFileList.focus
		#markup = ("std", widget.base_widget.origTxt)
		#widget.base_widget.set_label(markup)

		#widget = self.widgetFileList.body[newFocus]
		#markup = ("std_f", widget.base_widget.origTxt)
		#widget.base_widget.set_label(markup)
		widget = self.widgetFileList.body[newFocus]

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
		return True

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

			#markup = ur.terminal2markup(line, 0)
			#markupF = ur.terminal2markup(line, 1)
			markup = ("std", line)
			markupF = ('std_f', line)

			btn = ur.genBtn(markup, markupF, self.cbFileSelect, len(self.widgetFileList.body) == 0)
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

		# content
		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([], None)))
		self.widgetCmdList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([], None))) # 이거 파일목록 아래에뭔가 보여주는건데..
		self.widgetContent = urwid.Pile([self.widgetFileList, ('pack', urwid.Divider('-')), (8, self.widgetCmdList)])
		self.widgetContent.isShow = True

		# extra
		self.widgetWorkLabel = urwid.Text("< Workspace >")
		self.widgetWorkList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([], None)))
		self.widgetTempLabel = urwid.Text("< Temp >")
		self.widgetTempList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([], None)))
		self.widgetExtraList = urwid.Pile([("pack", self.widgetWorkLabel), self.widgetWorkList, ("pack", self.widgetTempLabel), self.widgetTempList])

		# main frame + input
		self.title = ">> dc V%s" % g.version
		self.headerText = urwid.Text(self.title)
		self.widgetFrame = urwid.Columns([(100, self.widgetContent), (20, self.widgetExtraList)])
		self.edInput = ur.genEdit("$ ", "", lambda edit,text: self.onInputChanged(edit, text))
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText, footer=self.edInput)

		self.cmd = ""
		self.mode = ""  # d면 등록된 폴더만 표시
		self.gitBranch = None
		self.dcdata = None
		self.lastPath = None

		# work space
		pp = os.getcwd()
		self.workList = [pp]
		self.workPt = 0


	def init(self):
		self.cmdShow([])  # hide extra panel
		self.changePath(os.getcwd())
		return True

	# 이거 용도가 뭔지 까먹었다. lstItem을 보여주는건데... 아마... 부가기능인듯...
	def cmdShow(self, lstItem):
		isShow = len(lstItem) > 0
		if isShow != self.widgetContent.isShow: # 이거 자체는 왼쪽 컨텐츠다.
			self.widgetContent.isShow = isShow
			if isShow:
				#self.widgetContent.contents[1] = (self.widgetContent.contents[1][0], (urwid.widget.PACK, None))
				self.widgetContent.contents[1] = (urwid.Divider('-'), (urwid.widget.PACK, None))
				self.widgetContent.contents[2] = (self.widgetContent.contents[2][0], (urwid.widget.GIVEN, 8))
			else:
				#self.widgetContent.contents[1] = (self.widgetContent.contents[1][0], (urwid.widget.GIVEN, 0))  # 이게 잘안된다. 아마 divider는 pack만 지원하는듯
				self.widgetContent.contents[1] = (urwid.Pile([]), (urwid.widget.GIVEN, 0))
				self.widgetContent.contents[2] = (self.widgetContent.contents[2][0], (urwid.widget.GIVEN, 0))

		if not isShow:
			return

		# list
		lstItem = [ ("std", x, None) for x in lstItem ]
		refreshBtnListMarkupTuple(lstItem, self.widgetCmdList, lambda btn: self.onFileSelected(btn))

	def onInputChanged(self, edit, text):
		if self.cmd == "find" or self.cmd == "goto":
			self.fileRefresh(text)

	def gotoRefresh(self, newText):
		filterStr = self.edInput.get_edit_text() if newText is None else newText
		if filterStr != "":
			lstPath = g.regFindItems(filterStr)
		else:
			lstPath = g.regList[:]

		lst = [("greenfg", x["path"], x) for x in lstPath]

		self.headerText.set_text("%s - %d" % (self.title, len(lst)))
		refreshBtnListMarkupTuple(lst, self.widgetFileList, lambda btn: self.onFileSelected(btn))

	def fileRefresh(self, newText = None):
		if self.cmd == "goto":
			self.gotoRefresh(newText)
			return

		# filtering
		if self.cmd == "find":
			filterStr = self.edInput.get_edit_text() if newText is None else newText
		else:
			filterStr = ""

		pp = os.getcwd()
		# TODO: use scandir

		self.dcdata = None
		lst = []	# name, order
		for item in os.listdir(pp):
			if item == ".dcdata":
				self.dcdataLoad()
				continue
			else:
				lst.append( (item, 0) )

		lst2 = []
		if filterStr != "":
			for x in lst:
				ss = x[0].lower()
				fil = filterStr.lower()
				if ss.startswith(fil):
					lst2.append((x[0], 2))
				elif fil in ss:
					lst2.append((x[0], 1))
				else:
					lst2.append((x[0], 0))
		else:
			# 등록된 폴더 우선
			regPathList = [ii['path'] for ii in g.regList]
			for x in lst:
				full = os.path.join(pp, x[0])
				if full in regPathList:
					lst2.append((x[0], 1))
				else:
					lst2.append((x[0], 0))
		lst = lst2

		# list
		def osStat(pp):
			try:
				return os.stat(pp)
			except Exception as e:
				return None

		# name, osStat, order
		lst2 = [ (x[0], osStat(os.path.join(pp, x[0])), x[1]) for x in lst]
		#if filterStr != "":
			#lst2.sort(key=lambda x: -1 if x[2] == 1 else 1)
		lst2.sort(key=lambda ii: -ii[2])

		# registered list only
		if self.dcdata is not None and self.mode != "":
			def __check(fname):
				dcItem = self.dcdataGet(fname)
				if dcItem is None:
					return self.mode == "d1"
				else:
					return dcItem["type"] == "S"

			lst2 = [ (x[0], x[1]) for x in lst2 if __check(x[0]) ]

		# dir sort
		def __sortStMode(stMode):
			if stMode is None:
				return 2
			elif stat.S_ISDIR(stMode.st_mode):
				return -1
			else:
				return 1
				
		lst2.sort(key=lambda x: __sortStMode(x[1]))
		lst2.insert(0, ("..", None, 0))

		#itemList = [ (os.path.basename(x[0]), x[1], x[2]) for x in lst2]
		# mstd, name, fattr
		def gen(x):
			if x[0] == "..":
				isDir = True
			elif x[1] is None:
				isDir = False
			else:
				isDir = stat.S_ISDIR(x[1].st_mode)

			mstd = None
			if filterStr != "":
				if x[2] == 0:
					mstd = "grayfg"
				elif x[2] == 1:
					mstd = "bluefg"
				elif x[2] == 2:
					mstd = "cyanfg"
			else:
				if x[2] == 1:
					mstd = 'bold'
				else:
					if isDir:
						dcItem = self.dcdataGet(x[0])
						if dcItem is not None:
							if dcItem["type"] == "S":
								mstd = "bluefgb"
							else:
								mstd = "grayfg"
					else:
						dcItem = self.dcdataGet(x[0])
						if dcItem is not None:
							if dcItem["type"] == "S":
								mstd = "bold"
							else:
								mstd = "grayfg"

					mstd = 'greenfg' if isDir else 'std'

			return mstd, x[0], x[1]

		# status
		itemList = list(map(gen, lst2))
		status = ""
		item = g.regFindByPath(pp)
		if item is not None:
			status = "*"
			if "repo" in item and item["repo"]:
				status += "+"
			status = "(%s)" % status

		# git post
		gitSt = ""
		if self.gitBranch is not None:
			cntStaged = 0
			cntModified = 0
			cntUntracked = 0
			gitItemList = git.statusFileList()
			for gitItem in gitItemList:
				if gitItem[1] == "s":
					cntStaged += 1
				elif gitItem[1] == "?":
					cntUntracked += 1
				else:
					cntModified += 1

				name = ur.termianl2plainText(gitItem[0])[3:]
				def gen2(x):
					#print("target - [%s] - %s" % (x[2], name))
					if x[1] == name:
						if gitItem[1] == "s":
							mstd = "bluefg"
						elif gitItem[1] == "?":
							mstd = "underline"
						else:
							mstd = "cyanfg"

						return mstd, x[1], x[2]
					else:
						return x

				itemList = list(map(gen2, itemList))

			ss1 = ""
			if cntStaged > 0:
				ss1 += "S:%d, " % cntStaged
			if cntModified > 0:
				ss1 += "M:%d, " % cntModified
			if cntUntracked > 0:
				ss1 += "?:%d, " % cntUntracked

			if ss1 != "":
				ss1 = ss1[:-2]

			gitSt = " - git(%s)" % ss1

		featureStr = "" if self.cmd is "" else "[%s]" % self.cmd
		featureExtra = ""
		if self.cmd == "find":
			featureExtra = ""
		ss = "%s%s - %s%s - %d%s %s" % (self.title, featureStr, pp, status, len(itemList)-1, gitSt, featureExtra)
		self.headerText.set_text(ss)

		if filterStr == "" and self.lastPath == pp:
			focusPos = self.widgetFileList.focus_position
		else:
			focusPos = 1

		if focusPos >= len(itemList):
			focusPos = 0

		self.lastPath = pp

		del self.widgetFileList.body[:]
		self.widgetFileList.body += ur.makeBtnListMarkup(itemList, lambda btn: self.onFileSelected(btn))
		self.widgetFileList.focus_position = focusPos

		# extra
		'''
		lst = []
		if filterStr != "":
			lst += g.findItems(filterStr)
			lst = [ x["path"] for x in lst ]

		self.extraShow(lst)
		'''

	def dcdataLoad(self):
		with open(".dcdata", "r") as fp:
			self.dcdata = json.load(fp)

	def dcdataSave(self):
		if self.dcdata is None:
			os.remove(".dcdata")
			return

		with open(".dcdata", "w") as fp:
			json.dump(self.dcdata, fp)

	def dcdataGet(self, fname):
		if self.dcdata is None:
			return None

		for item in self.dcdata:
			if item["name"] == fname:
				return item
		return None

	def dcdataAdd(self, fname, obj):
		if self.dcdata is None:
			self.dcdata = []

		obj["name"] = fname
		self.dcdata.append(obj)

	def dcdataRemove(self, item):
		self.dcdata.remove(item)
		if len(self.dcdata) == 0:
			self.dcdata = None


	def onFileSelected(self, btn):
		pass

	def changePath(self, pp, newCmd=""):
		if not os.path.isdir(pp):
			return False

		pp = os.path.realpath(pp)
		os.chdir(pp)

		# check git repo
		try:
			ss = subprocess.check_output(["git", "branch", "--color=never"], stderr=subprocess.DEVNULL).decode()
			name = re.search(r"^\*\s(\w+)", ss, re.MULTILINE)
			self.gitBranch = name.group(1)
		except subprocess.CalledProcessError:
			self.gitBranch = None

		self.workList[self.workPt] = pp
		self.workRefresh()

		self.mode = ""  # 모드도 초기화

		# filter상태도 클리어하는게 맞나?
		self.inputSet(newCmd)
		self.edInput.set_edit_text("")
		self.fileRefresh()

	def inputSet(self, status):
		"""
		:param status: filter,
		:return:
		"""
		self.cmd = status
		if status == "":
			self.mainWidget.set_focus("body")
		else:
			self.mainWidget.set_focus("footer")

		self.edInput.set_edit_text("")
		self.edInput.set_caption("%s%s$ " % ("" if self.gitBranch is None else "[%s] " % self.gitBranch, self.cmd))

	def regToggle(self, pp):
		item = g.regFindByPath(pp)
		if item is None:
			self.regAdd(pp)
		else:
			self.regRemove(pp)

	def regRemove(self, pp):
		item = g.regFindByPath(pp)
		if item is None:
			ur.popupMsg("Remove the folder", "The path is not registered\n%s" % pp, 60)
			return

		def onOk():
			g.regList.remove(item)
			g.configSave()
			self.fileRefresh()
			ur.popupMsg("Remove the folder", "The path is registerted successfully\n%s" % pp, 60)

		ur.popupAsk("Remove the folder", "Do you want to delete that folder?\n%s" % pp, onOk)

	def regAdd(self, pp):
		name = os.path.basename(pp)
		g.regList.append(dict(names=[name], path=pp, groups=[], repo=False))
		g.configSave()
		self.fileRefresh()
		ur.popupMsg("Regiter the folder", "The path is registerted successfully\n%s" % pp, 60)

	def inputFilter(self, keys, raw):
		if g.loop.widget != g.dialog.mainWidget:
			return keys

		if self.cmd == "find":
			# ctrl+j는 enter, alt+시리즈는 안오고.. 그냥 shift+JKH를 쓴다
			if ur.filterKey(keys, "up"):
				self.widgetFileList.focusPrevious()
			elif ur.filterKey(keys, "down"):
				self.widgetFileList.focusNext()
			elif ur.filterKey(keys, "J"):
				self.widgetFileList.focusNext()
			elif ur.filterKey(keys, "K"):
				self.widgetFileList.focusPrevious()
			elif ur.filterKey(keys, "U"):
				self.changePath("..", "find")
			elif ur.filterKey(keys, "H"):
				self.changePath(self.getFocusPath(), "find")
			elif ur.filterKey(keys, "enter"):
				# self.mainWidget.set_focus("body")
				self.changePath(self.getFocusPath())  # 바로 이동 + find는 푼다

			elif ur.filterKey(keys, "C"):
				self.doCommit()

			elif ur.filterKey(keys, "ctrl ^"):
				if self.mainWidget.get_focus() == "body":
					pass
				elif self.mainWidget.get_focus() == "footer":
					# find cmd
					ss = self.edInput.get_edit_text()
					self.inputSet("")
					self.doFind(ss)
					return

		elif ur.filterKey(keys, "enter"):
			if self.mainWidget.get_focus() == "body":
				self.changePath(self.getFocusPath())
				return
			else:
				if self.cmd == "goto":
					self.changePath(self.getFocusPath())
					return
				elif self.cmd == "shell":
					ss = self.edInput.get_edit_text()
					self.inputSet("")

					g.loop.stop()
					systemRet(ss)
					input("Enter to return...")
					g.loop.start()
					self.fileRefresh()

				elif self.cmd == "cmd":
					ss = self.edInput.get_edit_text()
					self.inputSet("")

					if ss == "list":
						self.doFolderList()
					elif ss == "reg":
						pp = os.getcwd()
						item = g.regFindByPath(pp)
						if item is not None:
							# already registered
							ur.popupMsg("Regiter the folder", "The path is already registerted\n%s" % pp, 60)
							return

						# add
						self.regAdd(pp)
						return

					elif ss == "del":
						pp = os.getcwd()
						self.regRemove(pp)
						return

					elif ss == "set repo":
						pp = os.getcwd()
						item = g.regFindByPath(pp)
						if item is None:
							# no item
							ur.popupMsg("Set repo status", "The path is no registered\n%s" % pp, 60)
							return

						# set repo
						item["repo"] = not item["repo"] if "repo" in item else True

						g.configSave()
						self.fileRefresh()
						ur.popupMsg("Set repo status", "The path is set as %s\n%s" % ("Repo" if item["repo"] else "Not Repo", pp), 60)
						return
					else:
						ur.popupMsg("Command", "No valid cmd\n -- %s" % ss)

			# 이거 뭐하는 코드지?
			#item = self.widgetCmdList.focus
			#pp = item.original_widget.get_label()
			#self.changePath(pp)

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

	def getFocusName(self):
		btn = self.widgetFileList.focus
		fname = btn.base_widget.get_label()
		return fname

	def getFocusPath(self):
		pp = os.getcwd()
		fname = self.getFocusName()
		return os.path.join(pp, fname)

	def workNew(self):
		pp = os.getcwd()
		self.workList.append(pp)
		self.workPt += 1
		self.workRefresh()

	def workRemove(self):
		if len(self.workList) <= 1:
			return

		del self.workList[self.workPt]
		if self.workPt >= len(self.workList)-1:
			self.workPt = len(self.workList)-1

		self.workGo()

	def workMove(self, add):
		if add == 0:
			return

		elif add < 0:
			if self.workPt == 0:
				return
			self.workPt -= 1
		else:
			if self.workPt+1 >= len(self.workList):
				return

			self.workPt += 1

		self.workGo()

	def workGo(self):
		# control refresh and go
		self.workRefresh()
		item = self.widgetWorkList.focus
		x = item.original_widget.attr
		os.chdir(x)
		self.fileRefresh()


	def workRefresh(self):
		del self.widgetWorkList.body[:]

		# std, focus, text, attr
		itemList =  [ ("std", os.path.basename(x), x) for x in self.workList ]
		self.widgetWorkList.body += ur.makeBtnListMarkup(itemList, lambda btn: self.onFileSelected(btn))
		self.widgetWorkList.focus_position = self.workPt

	def doCommit(self):
		def onExit():
			g.doSetMain(self)

		if self.gitBranch is None:
			ur.popupMsg("Error", "Not git repository")
			return

		dlg = mDlgMainGitStatus(onExit)
		g.doSetMain(dlg)

	def unhandled(self, key):
		if key == 'f4' or key == "q":
			g.savePath(os.getcwd())
			raise urwid.ExitMainLoop()

		elif key == "f1":
			# help
			pass

		elif key == "f5":
			self.fileRefresh()
			return

		elif key == "f":  # filter
			self.inputSet("find")
			return

		elif key == "R":
			pp = self.getFocusPath()
			self.regToggle(pp)
			return

		elif key == "g":  # go
			self.inputSet("goto")
			self.fileRefresh()
			return

		elif key == "d":
			if self.mode == "":
				self.mode = "d1"
			elif self.mode == "d1":
				self.mode = "d2"
			else:
				self.mode = ""

			self.fileRefresh()

		elif key == "/":  # cmd
			self.inputSet("cmd")
			return

		elif key == "s": # shell
			self.inputSet("shell")
			return

		elif key == "C": # git commit
			self.doCommit()
			return

		elif key == "F": # git update
			cur = os.getcwd()
			g.loop.stop()
			gr.actionUpdate(cur)
			input("Enter to return...")
			g.loop.start()
			self.fileRefresh()

		elif key == "P":
			g.loop.stop()
			print("fetching first...")
			try:
				git.fetch()
				g.gitPush()
			except Exception as e:
				print("Error - %s" % e)

			input("Enter to return...")
			g.loop.start()
			self.fileRefresh()

		elif key == "E":
			pp = self.getFocusPath()
			# /cygdrive/...을 제대로 인식못한다. 그냥 상대 경로로..
			name = os.path.basename(pp)

			g.loop.stop()
			systemRet("e %s" % name)
			g.loop.start()
			self.fileRefresh()

		elif key == "M" or key == "N":
			fname = self.getFocusName()
			item = self.dcdataGet(fname)

			ftype = "S" if key == "M" else "I"
			if item is None:
				self.dcdataAdd(fname, dict(type=ftype))
			else:
				if item["type"] == ftype:
					self.dcdataRemove(item)
				else:
					item["type"] = ftype

			self.dcdataSave()
			self.fileRefresh()

		#elif key == "ctrl h":
		#	ur.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")

		elif key == "meta right" or key == "meta l":
			self.workNew()
		elif key == "meta left" or key == "meta h":
			self.workRemove()

		elif key == "meta up" or key == "meta k":
			self.workMove(-1)
		elif key == "meta down" or key == "meta j":
			self.workMove(1)

		elif key == "j" or key == "J":   # we can't use ctrl+j since it's terminal key for enter replacement
			self.widgetFileList.focusNext()
		elif key == "k" or key == "K":
			self.widgetFileList.focusPrevious()
		elif key == "u" or key == "." or key == "U":
			self.changePath("..")

		elif key == "h" or key == "H":   # enter
			self.changePath(self.getFocusPath())

		elif key == "up":
			if self.cmd == "goto":
				self.widgetFileList.focusPrevious()
			else:
				self.mainWidget.set_focus("body")
		elif key == "down":
			if self.cmd == "goto":
				self.widgetFileList.focusNext()
			else:
				self.mainWidget.set_focus("body")

		elif key == "esc":
			self.mode = ""  # 모드도 초기화
			self.inputSet("")
			self.fileRefresh()

		'''
		elif type(key) == tuple:    # mouse
			pass
		else:
			self.mainWidget.set_focus("footer")
			#print(key)
			if len(key) == 1:
				#self.edInput.set_edit_text(self.edInput.get_edit_text()+key)
				self.edInput.insert_text(key)
		'''


	def doFolderList(self):
		def onExit():
			g.doSetMain(self)
			'''
			if not self.refreshFileList():
				g.loop.stop()
				print("No modified or untracked files")
				sys.exit(0)
			'''

		dlg = mDlgRegList(onExit)
		g.doSetMain(dlg)

	def doFind(self):
		pass

# 두칸씩 작은 오버레이로 띄우자
class mDlgFolderSetting(ur.cDialog):
	def __init__(self, onExit, item):
		super().__init__()
		self.onExit = onExit
		self.item = item

		self.header = ">> dc V%s - folder setting" % g.version
		self.headerText = urwid.Text(self.header)

		self.lbPath = urwid.Text("Path: %s" % item["path"])
		self.lbRepo = urwid.Text("Repo: ..")

		self.lbNames = urwid.Text("Names -----------")
		self.lbGroups = urwid.Text("Groups -----------")
		self.widgetListName = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([], None)))
		self.widgetListGroup = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal(["< No group >"], None)))

		#urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([], None)))
		self.lbHelp = urwid.Text("Insert: new name/group, Delete: remove name/group, R: toggle repo status")

		self.widgetFrame = urwid.LineBox(urwid.Pile(
			[("pack", self.headerText),
			("pack", self.lbPath),
			("pack", self.lbRepo),
            ("pack", self.lbNames), (8, self.widgetListName),
			('pack', urwid.Divider('-')),
            ("pack", self.lbGroups), (8, self.widgetListGroup),
			("pack", self.lbHelp)]))

		self.mainWidget = urwid.Overlay(urwid.Filler(self.widgetFrame), g.loop.widget, 'center', 80, 'middle', 30)

	def init(self):
		self.showInfo()
		return True

	def showInfo(self):
		self.lbRepo.set_text("Repo: %s" % ("O" if self.item["repo"] else "X"))

		names = self.item["names"]
		del self.widgetListName.body[:]
		self.widgetListName.body += ur.makeBtnListTerminal(names, None)

		groups = self.item["groups"]
		if len(groups) > 0:
			del self.widgetListGroup.body[:]
			self.widgetListGroup.body += ur.makeBtnListTerminal(groups, None)

		#self.widgetFrame.set_focus(self.widgetContent)

	def unhandled(self, key):
		if key == 'f4' or key == "q":
			self.close()
		elif key == "r":
			self.item["repo"] = not self.item["repo"]
			g.configSave()
			self.showInfo()

		elif key == "insert":
			focusWidget = self.widgetFrame.original_widget.get_focus()
			if focusWidget == self.widgetListName:
				def onOk(ss):
					self.item["names"].append(ss)
					g.configSave()
					self.showInfo()

				ur.popupInput("Input new name", "", onOk, width=60)

			elif focusWidget == self.widgetListGroup:
				def onOk(ss):
					self.item["groups"].append(ss)
					g.configSave()
					self.showInfo()

				ur.popupInput("Input new group", "", onOk, width=60)

		elif key == "delete":
			focusWidget = self.widgetFrame.original_widget.get_focus()
			if focusWidget == self.widgetListName:
				ss = self.widgetListName.focus.original_widget.get_label()
				def onOk():
					self.item["names"].remove(ss)
					g.configSave()
					self.showInfo()

				ur.popupAsk("Remove Name", "[%s] will be deleted. Are you sure?" % ss, onOk)

			elif focusWidget == self.widgetListGroup:
				ss = self.widgetListGroup.focus.original_widget.get_label()
				def onOk():
					self.item["groups"].remove(ss)
					g.configSave()
					self.showInfo()

				ur.popupAsk("Remove Group", "[%s] will be deleted. Are you sure?" % ss, onOk)


class mDlgRegList(ur.cDialog):
	def __init__(self, onExit):
		super().__init__()

		self.onExit = onExit
		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([], None)))
		#self.widgetFileList.setFocusCb(lambda newFocus: self.onFileFocusChanged(newFocus))
		self.widgetContent = ur.mListBox(urwid.SimpleListWalker(ur.makeTextList(["< Nothing to display >"])))
		#self.widgetContent.isViewContent = True

		self.header = ">> dc V%s - folder list" % g.version
		self.headerText = urwid.Text(self.header)

		#self.widgetFrame = urwid.Pile(
		#	[(15, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.widgetFrame = urwid.AttrMap(self.widgetFileList, 'std')
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)

		#self.cbFileSelect = lambda btn: self.onFileSelected(btn)
		self.onFileSelected = lambda btn: None

		#self.content = ""
		#self.selectFileName = ""

	def init(self):
		self.refreshFile()
		return True

	def refreshFile(self):
		def getStatus(item):
			ss = ""
			if item["repo"]:
				ss = "(+)"

			ss += " - "
			for n in item["names"]:
				ss += n + ", "
			ss = ss[:-2]
			return ss

		itemList = [ (os.path.basename(x["path"]) + getStatus(x), x) for x in g.regList ]

		def gen(x):
			mstd = "greenfg" if "repo" in x[1] and x[1]["repo"] else "std"
			return mstd, x[0], x[1]

		# status
		itemList = list(map(gen, itemList))
		#self.headerText.set_text("%s - %s%s - %d" % (self.title, pp, status, len(itemList)))
		refreshBtnListMarkupTuple(itemList, self.widgetFileList, lambda btn: self.onFileSelected(btn))
		#del self.widgetFileList.body[:]
		#self.widgetFileList.itemCount = len(lst2)
		#self.widgetFileList.body += ur.makeBtnListTerminal( , None)

	def unhandled(self, key):
		if key == 'f4' or key == "q" or key == "esc":
			self.close()
		elif key == "j":  # we can't use ctrl+j since it's terminal key for enter replacement
			self.widgetFileList.focusNext()
		elif key == "k":
			self.widgetFileList.focusPrevious()
		elif key == "e" or key == "enter":
			item = self.widgetFileList.focus
			self.doEdit(item.original_widget.attr)

	def doEdit(self, item):
		def onExit():
			g.doSetMain(self)

		dlg = mDlgFolderSetting(onExit, item)
		g.doSetMain(dlg)


class mDlgMainGitStatus(ur.cDialog):
	def __init__(self, onExit=None):
		super().__init__()

		self.onExit = onExit
		self.selectFileName = ""

		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([("< No files >", None)], None)))
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
		if not self.refreshFileList():
			print("No modified or untracked files")
			return False

		return True

	def onFileSelected(self, btn):
		# why btn.get_label() is impossible?
		label = btn.original_widget.get_label()
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
		itemList = git.statusFileList()
		if len(itemList) <= 0:
			return False

		focusIdx = self.widgetFileList.focus_position
		refreshBtnListTerminal(itemList, self.widgetFileList, lambda btn: self.onFileSelected(btn))
		size = len(self.widgetFileList.body)

		focusIdx += focusMove
		if focusIdx >= size:
			focusIdx = size-1
		#self.widgetFileList.focus_position = focusIdx
		self.widgetFileList.set_focus(focusIdx)
		self.onFileSelected(self.widgetFileList.focus)	# auto display
		return True


	def gitGetStagedCount(self):
		cnt = 0
		for item in self.widgetFileList.body:
			if "s" in item.original_widget.attr:	# greenfg
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
			self.close()
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
				if not self.refreshFileList():
					if getattr(self, "onExit") and self.onExit is not None:
						self.close()
						return
					else:
						g.loop.stop()
						print("No modified or untracked files")
						sys.exit(0)

				g.doSetMain(self)

			# check staged data 
			n = self.gitGetStagedCount()
			if n == 0:
				ur.popupMsg("Alert", "There is no staged file to commit")
				return
				
			dlg = mGitCommitDialog(onExit)
			g.doSetMain(dlg)

		elif key == "h":
			ur.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")


class mGitCommitDialog(ur.cDialog):
	themes = [("greenfg", "greenfg_f"), ("std", "std_f")]
	
	def __init__(self, onExit):
		super().__init__()

		self.selectFileName = ""

		self.onExit = onExit
		self.edInput = ur.genEdit("Input commit message => ", "", lambda edit,text: self.onMsgChanged(edit,text))
		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([("< No files >", None)], None)))
		self.widgetContent = ur.mListBox(urwid.SimpleListWalker(ur.makeTextList(["< Nothing to display >"])))

		self.headerText = urwid.Text(">> Commit...")
		self.widgetFrame = urwid.Pile([("pack", self.edInput), (8, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)
		
		self.refreshFileList()
		self.widgetFrame.set_focus(self.edInput)

	def onMsgChanged(self, edit, text):
		pass
		
	def _applyFileColorTheme(self, widget, isFocus=0):
		theme = self.themes[0 if widget.original_widget.attr == "s" else 1]
		widget.original_widget.set_label((theme[isFocus], widget.original_widget.origTxt))

	def onFileSelected(self, btn):
		# why btn.get_label() is impossible?
		label = btn.original_widget.get_label()
		self.selectFileName = btn.original_widget.get_label()
		#g.headerText.set_text("file - " + label)
		
		# display
		btnType = btn.original_widget.attr
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
		itemList = [ (self.themes[0][0], x, "s") for x in fileList.split("\n") if x.strip() != "" ]
		self.widgetFileList.body += ur.makeBtnListMarkup(itemList, lambda btn: self.onFileSelected(btn))

		# general file list
		fileList = system("git diff --name-only")
		itemList = [ (self.themes[1][0], x, "c") for x in fileList.split("\n") if x.strip() != ""  ]
		self.widgetFileList.body += ur.makeBtnListMarkup(itemList, lambda btn: self.onFileSelected(btn), False)

		#for widget in self.widgetFileList.body:
		#	self._applyFileColorTheme(widget, 0)

		if len(self.widgetFileList.body) == 0:
			self.widgetFileList.body += ur.makeBtnListTerminal([("< Nothing >", None)], None, False)

		#self.onFileFocusChanged(self.widgetFileList.focus_position)
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
			self.close()
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
				self.close()
					
			ur.popupAsk("Git Commit", "Do you want to commit all modification?", onCommit)
			
		elif key == "enter":
			# commit
			tt = self.edInput.get_edit_text()
			ss = system("git commit -m \"%s\"" % tt)
			#print(ss)
			self.close()

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
	label = btn.original_widget.get_label()
	return label[2:].strip()

# "??" - untracked file
def gitFileBtnType(btn):
	label = btn.original_widget.get_label()
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
		fname = fname[pt+4:]
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
	g.loop = urwid.MainLoop(dlg.mainWidget, ur.palette, urwid.raw_display.Screen(),
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
		item = g.regFindByName(target)
		os.chdir(item["path"])
		cmds = cmds[:targetItemIdx] + cmds[targetItemIdx+1:]

	uiMain(dlgClass, lambda writeFd: subprocess.Popen(cmds, bufsize=0, stdout=writeFd, close_fds=True))


class Gr(object):
	def __init__(self):
		self.isInit = False
		self.repoList = [dict(name=["test"], path="")]
		
	def init(self):
		self.repoList = [repo for repo in g.regList if "repo" in repo and repo["repo"]]
		self.isInit = True
		
	def repoAllName(self):
		return [repo["names"][0] for repo in self.repoList]
		
	def action(self, action, target):
		if not self.isInit:
			self.init()

		if target is not None:
			action(self, target)

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
			if name in repo["names"]:
				return repo
				
		raise Exception("Can't find repo[name:%s]" % name)

	def getRepoPath(self, name):
		repo = self.getRepo(name)
		path = repo["path"]
		return path
				
	def changePath(self, name):
		if name.startswith("/"):
			path = name
		else:
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

	def actionUpdate(self, target):
		print("fetch......")
		gr.action(Gr.fetch, target)

		print("merge......")
		gr.action(Gr.mergeSafe, target)

		print("status......")
		gr.action(Gr.statusComponent, target)

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

		# allow the repo that no registerted
		if not name.startswith("/"):
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
		cmd = ""	# basic cmd
	else:
		cmd = sys.argv[1]
		
	removeEmptyArgv()

	target = None
	if len(sys.argv) >= 3:
		target = sys.argv[2]
		if target == ".":
			# current repo
			cur = os.getcwd() + "/"

			# allow the repo that isn't registerted
			target = cur

			'''
			for repo in gr.repoList:
				repoPath = os.path.realpath(repo["path"])
				if cur.startswith(repoPath+"/"):
					second = repo["names"][0]
					break
			if second == ".":
				self.log(0, "Current path[%s] is not git repo." % cur)
				return
			'''

	if cmd == "":
		uiMain(mDlgMainDc)
		return

	elif cmd == "push":
		print("fetching first...")
		git.fetch()
		g.gitPush()
		return
		
	elif cmd == "ci":
		uiMain(mDlgMainGitStatus)
		return
		
	elif cmd == "list":
		g.regListPrint()
		return
		
	elif cmd == "config":
		g.savePath("~/.devcmd")
		return
		
	elif cmd == "which":
		ss, status = systemSafe(" ".join(['"' + c + '"' for c in sys.argv[1:]]))
		print(ss)
		print("goto which path...")
		g.savePath(os.path.dirname(ss))
		return
	
	elif cmd == "find":
		# dc find . -name "*.py"
		cmds = sys.argv[1:]
		doSubCmd(cmds, mDlgMainFind)
		return
		
	elif cmd == "findg":
		pp = sys.argv[2]
		if "*" not in pp:
			pp = "*"+pp+"*"

		cmds = ["find", ".", "-name", pp]
		doSubCmd(cmds, mDlgMainFind, 4)
		return
		
	elif cmd == "ack":
		# dc ack printf
		cmds = sys.argv[1:]
		cmds.insert(1, "--group")
		cmds.insert(1, "--color")
		doSubCmd(cmds, mDlgMainAck)
		return
		
	elif cmd == "ackg":
		# dc ack printf
		cmds = ["ack"] + sys.argv[2:]
		cmds.insert(1, "--group")
		cmds.insert(1, "--color")
		doSubCmd(cmds, mDlgMainAck, 4)
		return
		
	elif cmd == "st":
		gr.action(Gr.statusComponent, target)
		return
		
	elif cmd == "fetch":
		gr.action(Gr.fetch, target)
		return
		
	elif cmd == "merge":
		gr.action(Gr.mergeSafe, target)
		return
		
	elif cmd == "update":
		gr.actionUpdate(target)
		return

	#print("target - %s" % target)
	g.cd(cmd)
	return 1
	

if __name__ == "__main__":
	try:
		ret = run()
	except ErrFailure as e:
		print(e)
		sys.exit(1)
	

