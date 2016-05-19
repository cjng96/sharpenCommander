# coding: utf-8
#!/usr/bin/env python3

import subprocess

import os
import sys
import select

import tool
from tool import git, system, systemSafe, systemRet

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

	def __init__(self, body):
		super().__init__(body)
		
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
		
	def mouse_event(self, size, event, button, col, row, focus):
		if event == "mouse press":
			if button == 4:	# up
				for i in range(3):
					self.scrollUp()
			
			elif button == 5:	# down
				for i in range(3):
					self.scrollDown()
					


def refreshBtnList(content, listBox, onClick):
	del listBox.body[:]
	if content.strip() == "":
		contentList = ["< Nothing >"]
		listBox.itemCount = 0
	else:
		contentList = content.split("\n")
		listBox.itemCount = len(contentList)
		
	listBox.body += Urwid.makeBtnList(contentList, onClick)


class cDialog():
	def __init__(self):
		self.mainWidget = None
	
	def unhandled(self, key):
		pass 
		
	def inputFilter(self, keys, raw):
		return keys

class AckFile:
	def __init__(self, fnameTerminal):
		self.fname = Urwid.termianl2plainText(fnameTerminal)
		#self.fnameMarkup = Urwid.terminal2markup(fnameTerminal, 0)
		#self.fnameOrig = fnameTerminal

		self.lstLine = []	
		
	def getTitleMarkup(self, focus=False):
		themeTitle = "greenfg" if not focus else "greenfg_f"
		themeCount = "std" if not focus else "std_f"  
		return [(themeTitle, self.fname), (themeCount, "(%d)" % len(self.lstLine))]


class mDlgMainAck(cDialog):
	def __init__(self):
		super().__init__()

		self.widgetFileList = mListBox(urwid.SimpleFocusListWalker(Urwid.makeBtnList([], None)))
		self.widgetFileList.body.set_focus_changed_callback(lambda new_focus: self.onFileFocusChanged(new_focus))
		self.widgetContent = mListBox(urwid.SimpleListWalker(Urwid.makeTextList([])))

		self.header = ">> dc V%s - ack-grep - q/F4(Quit),<-/->(Prev/Next file),Enter(goto),E(edit)..." % g.version
		self.headerText = urwid.Text(self.header)
		self.widgetFrame = urwid.Pile([(15, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)
		
		self.cbFileSelect = lambda btn: self.onFileSelected(btn)
		self.buf = ""
		self.lstContent = []
		
	def btnUpdate(self, btn, focus):
		btn.base_widget._label.set_text(btn.afile.getTitleMarkup(focus))
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
	
		if "left" in keys:
			self.widgetFileList.focusPrevious()
			return [ c for c in keys if c != "left" ]
			
		if "right" in keys:
			self.widgetFileList.focusNext()
			return [ c for c in keys if c != "right" ]
			
		if "down" in keys:
			self.widgetContent.scrollDown()
			return [ c for c in keys if c != "down" ]

		if "up" in keys:
			self.widgetContent.scrollUp()
			return [ c for c in keys if c != "up" ]
			
		if "enter" not in keys:
			return keys

		self.onFileSelected(self.widgetFileList.focus)
		
	def recvData(self, data):
		ss = data.decode("UTF-8")
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
				
				btn = Urwid.genBtnMarkup(afile.getTitleMarkup(False), self.cbFileSelect, len(self.widgetFileList.body) == 0)
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
				txt = Urwid.genText(line)
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
			Urwid.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")
	
		
	

class mDlgMainFind(cDialog):
	def __init__(self):
		super().__init__()

		self.widgetFileList = mListBox(urwid.SimpleFocusListWalker(Urwid.makeBtnList([], None)))
		self.widgetFileList.body.set_focus_changed_callback(lambda new_focus: self.onFileFocusChanged(new_focus))
		self.widgetContent = mListBox(urwid.SimpleListWalker(Urwid.makeTextList(["< Nothing to display >"])))

		self.header = ">> dc V%s - find - q/F4(Quit),<-/->(Prev/Next file),Enter(goto),E(edit)..." % g.version
		self.headerText = urwid.Text(self.header)
		self.widgetFrame = urwid.Pile([(15, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)
		
		self.cbFileSelect = lambda btn: self.onFileSelected(btn)
		self.content = ""

	def onFileFocusChanged(self, new_focus):
		# old widget
		widget = self.widgetFileList.focus
		#markup = Urwid.terminal2markup(widget.base_widget.origText, 0)
		markup = ("std", widget.base_widget.origText)
		widget.base_widget._label.set_text(markup)

		widget = self.widgetFileList.body[new_focus]
		#markup = Urwid.terminal2markup(widget.base_widget.origText, 1)
		markup = ("std_f", widget.base_widget.origText)
		widget.base_widget._label.set_text(markup)

		self.selectFileName = gitFileBtnName(widget)

		try:
			with open(self.selectFileName, "r", encoding="UTF-8") as fp:
				ss = fp.read()
		except UnicodeDecodeError:
			ss = "No utf8 file[size:%d]" % os.path.getsize(self.selectFileName) 
			
		ss = ss.replace("\t", "    ")
			
		del self.widgetContent.body[:]
		self.widgetContent.body += Urwid.makeTextList(ss.splitlines())
		self.widgetFrame.set_focus(self.widgetContent)

	def onFileSelected(self, btn):
		self.selectFileName = gitFileBtnName(btn)
		pp = os.path.dirname(os.path.join(os.getcwd(), self.selectFileName))
		g.savePath(pp)
		raise urwid.ExitMainLoop()
		
	def inputFilter(self, keys, raw):
		if "enter" not in keys:
			return keys

		#idx = keys.index("enter")
		self.onFileSelected(self.widgetFileList.focus)
		#return keys[:idx-1]+keys[idx+1:]
		
		
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
			
			btn = Urwid.genBtn(line, self.cbFileSelect, len(self.widgetFileList.body) == 0)  
			self.widgetFileList.body.append(btn)
			
		return True
			

	def unhandled(self, key):
		if key == 'f4' or key == "q":
			raise urwid.ExitMainLoop()
		elif key == 'left' or key == "[":
			self.widgetFileList.focusPrevious()
		elif key == 'right' or key == "]":
			self.widgetFileList.focusNext()
		elif key == "e" or key == "E":
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)

			g.loop.stop()
			systemRet("vim %s" % fname)
			g.loop.start()
			
		elif key == "h":
			Urwid.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")
	
	
class mDlgMainGitStatus(cDialog):
	def __init__(self):
		super().__init__()

		self.widgetFileList = mListBox(urwid.SimpleFocusListWalker(Urwid.makeBtnList(["< No files >"], None)))
		self.widgetFileList.body.set_focus_changed_callback(lambda new_focus: self.onFileFocusChanged(new_focus))
		self.widgetContent = mListBox(urwid.SimpleListWalker(Urwid.makeTextList(["< Nothing to display >"])))

		self.headerText = urwid.Text(">> dc V%s - q/F4(Quit),<-/->(Prev/Next file),A(Add),P(Prompt),R(Reset),D(drop),C(Commit),I(Ignore)" % g.version)
		self.widgetFrame = urwid.Pile([(8, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)

		g.gitRoot = system("git rev-parse --show-toplevel")
		g.curPath = os.getcwd()
		g.relRoot = "./"
		if g.gitRoot != g.curPath:
			g.relRoot = os.path.relpath(g.gitRoot, g.curPath)
			

	def onFileFocusChanged(self, new_focus):
		# old widget
		widget = self.widgetFileList.focus
		markup = Urwid.terminal2markup(widget.base_widget.origText, 0)
		widget.base_widget._label.set_text(markup)

		widget = self.widgetFileList.body[new_focus]
		markup = Urwid.terminal2markup(widget.base_widget.origText, 1)
		widget.base_widget._label.set_text(markup)

	def onFileSelected(self, btn):
		# why btn.get_label() is impossible?
		label = btn.base_widget.get_label()
		self.selectFileName = gitFileBtnName(btn)
		#g.headerText.set_text("file - " + label)
		
		# display
		if label == "< Nothing >":
			ss = label
		elif label.startswith("?? "):
			try:
				with open(self.selectFileName, "r", encoding="UTF-8") as fp:
					ss = fp.read()
			except UnicodeDecodeError:
				#Urwid.popupMsg("Encoding", "Encoding error[%s]" % self.selectFileName);
				ss = "No utf8 file[size:%d]" % os.path.getsize(self.selectFileName) 
				
		else:
			ss = system("git diff --color \"%s\"" % self.selectFileName)
			
		ss = ss.replace("\t", "    ")
			
		del self.widgetContent.body[:]
		self.widgetContent.body += Urwid.makeTextList(ss.splitlines())
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

	def inputFilter(self, keys, raw):
		if g.loop.widget != g.dialog.mainWidget:
			return keys
		
		if "left" in keys:
			self.widgetFileList.focusPrevious()
			self.refreshFileContentCur()
			return [ c for c in keys if c != "left" ]
			
		if "right" in keys:
			self.widgetFileList.focusNext()
			self.refreshFileContentCur()
			return [ c for c in keys if c != "right" ]
			
		return keys

	def unhandled(self, key):
		if key == 'f4' or key == "q":
			raise urwid.ExitMainLoop()
		elif key == 'k':
			self.widgetContent.scrollUp()
		elif key == 'j':
			self.widgetContent.scrollDown()
		elif key == "[":
			self.widgetFileList.focusPrevious()
			self.refreshFileContentCur()
		elif key == "]":
			self.widgetFileList.focusNext()
			self.refreshFileContentCur()
			
		elif key == "A":
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			system("git add %s" % fname)
			self.refreshFileList(1)
			
		elif key == "P":
			def onPrompt():
				g.loop.stop()
				systemRet("git add -p %s" % fname)
				g.loop.start()
				self.refreshFileList()
					
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			Urwid.popupAsk("Git add", "Do you want to add a file via prompt[%s]?" % fname, onPrompt)

		elif key == "R":
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			system("git reset %s" % fname)
			self.refreshFileList()
			
		elif key == "D":
			def onDrop():
				system("git checkout -- %s" % fname)
				self.refreshFileList()
					
			def onDelete():
				os.remove(fname)
				self.refreshFileList()
					
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			if gitFileBtnType(btn) == "??":
				Urwid.popupAsk("Git reset(f)", "Do you want to delete file[%s]?" % fname, onDelete)
			else:
				Urwid.popupAsk("Git reset(f)", "Do you want to drop file[%s]s modification?" % fname, onDrop)
		
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
					
			Urwid.popupAsk("Git commit", "Do you want to commit?", onCommit)

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
				
			dlg = mGitCommitDialog(onExit)
			g.dialog = dlg
			g.loop.widget = dlg.mainWidget
			
		elif key == "h":
			Urwid.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")

class mGitCommitDialog(cDialog):
	themes = [("greenfg", "greenfg_f"), ("std", "std_f")]
	
	def __init__(self, onExit):
		super().__init__()

		self.onExit = onExit
		self.edInput = Urwid.genEdit("Input commit message => ", "", lambda edit,text: self.onMsgChanged(edit,text))
		self.widgetFileList = mListBox(urwid.SimpleFocusListWalker(Urwid.makeBtnList(["< No files >"], None)))
		self.widgetFileList.body.set_focus_changed_callback(lambda new_focus: self.onFileFocusChanged(new_focus))
		self.widgetContent = mListBox(urwid.SimpleListWalker(Urwid.makeTextList(["< Nothing to display >"])))

		self.headerText = urwid.Text(">> Commit...")
		self.widgetFrame = urwid.Pile([("pack", self.edInput), (8, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)
		
		self.refreshFileList()
		self.widgetFrame.set_focus(self.edInput)

	def onMsgChanged(self, edit, text):
		pass
		
	def _applyFileColorTheme(self, widget, isFocus=0):
		theme = self.themes[0 if widget.base_widget.data == "s" else 1]
		widget.base_widget._label.set_text((theme[isFocus], widget.base_widget.origText))
	

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
		ss = system("git diff %s --color \"%s\"" % ("" if btnType == "c" else "--staged", pp))
		ss = ss.replace("\t", "    ")
			
		del self.widgetContent.body[:]
		self.widgetContent.body += Urwid.makeTextList(ss.split("\n"))
		self.widgetFrame.set_focus(self.widgetContent)

	def refreshFileContentCur(self):
		self.onFileSelected(self.widgetFileList.focus)

	def refreshFileList(self):
		del self.widgetFileList.body[:]

		# staged file list		
		fileList = system("git diff --name-only --cached")
		self.widgetFileList.body += Urwid.makeBtnList(fileList.split("\n"), 
			lambda btn: self.onFileSelected(btn), 
			lambda btn: setattr(btn, "data", "s"))

		# general file list
		fileList = system("git diff --name-only")
		self.widgetFileList.body += Urwid.makeBtnList(fileList.split("\n"), 
			lambda btn: self.onFileSelected(btn), 
			lambda btn: setattr(btn, "data", "c"))
			
		for widget in self.widgetFileList.body:
			self._applyFileColorTheme(widget, 0)
			
		if len(self.widgetFileList.body) == 0:
			self.widgetFileList.body += Urwid.makeBtnList(["< Nothing >"], None)
		else:
			self.onFileFocusChanged(self.widgetFileList.focus_position)
			self.onFileSelected(self.widgetFileList.focus)	# auto display

	def unhandled(self, key):
		if key == "q" or key == "Q" or key == "f4":
			self.onExit()
		elif key == 'k':
			self.widgetContent.scrollUp()
		elif key == 'j':
			self.widgetContent.scrollDown()
		elif key == "[":
			self.widgetFileList.focusPrevious()
			self.refreshFileContentCur()
		elif key == "]":
			self.widgetFileList.focusNext()
			self.refreshFileContentCur()
			
		elif key == "A":
			def onAdd():
				system("git add %s" % fname)
				self.refreshFileList()
					
			def onPrompt():
				g.loop.stop()
				systemRet("git add -p %s" % fname)
				g.loop.start()
				self.refreshFileList()
					
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			Urwid.popupAsk3("Git add", "Do you want to add a file[%s]?" % fname, "Add", "Prompt", "Cancel", onAdd, onPrompt)

		elif key == "R":
			def onReset():
				system("git reset %s" % fname)
				self.refreshFileList()
					
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			Urwid.popupAsk("Git reset", "Do you want to reset a file[%s]?" % fname, onReset)
			
		elif key == "D":
			def onDrop():
				system("git checkout --\"%s\"" % fname)
				self.refreshFileList()
					
			btn = self.widgetFileList.focus
			fname = gitFileBtnName(btn)
			Urwid.popupAsk("Git reset(f)", "Do you want to drop file[%s]s modification?" % fname, onDrop)
		
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
				text = self.edInput.get_edit_text()
				ss = system("git commit -a -m \"%s\"" % text[:-1])
				self.onExit()
					
			Urwid.popupAsk("Git Commit", "Do you want to commit all modification?", onCommit)
			
		elif key == "enter":
			# commit
			text = self.edInput.get_edit_text()
			ss = system("git commit -m \"%s\"" % text)
			#print(ss)
			self.onExit()

		elif key == "C":
			def onCommit():
				g.loop.stop()
				systemRet("git commit -a")
				g.loop.start()
				self.refreshFileList()
					
			Urwid.popupAsk("Git commit(all)", "Do you want to commit all content?", onCommit)
			
		elif key == "h":
			Urwid.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")


class Urwid:
	def termianl2plainText(ss):
		#source = "\033[31mFOO\033[0mBAR"
		st = ss.find("\x1b")
		if st == -1:
			return ss
			
		out = ""
		items = ss.split("\x1b")
		for at in items:
			if at == "":
				continue
			attr, text = at.split("m",1)
			if text != "":	# skip empty string
				out += text
			
		return out
		
	def terminal2markup(ss, invert=0):
		#source = "\033[31mFOO\033[0mBAR"
		table = {"[1":("bold",'bold_f'), "[4":("underline",'underline_f'),
			"[22":("std",'std_f'),
			"[24":("std",'std_f'),
			"[31":('redfg','redfg_f'), 
			"[32":('greenfg', "greenfg_f"), 
			"[33":('yellowfg', "yellowfg_f"), 
			"[36":('cyanfg', "cyanfg_f"), 
			"[41":("redbg", "regbg_f"),
			"[1;31":("redfg_b", "redfg_bf"), 
			"[1;32":("greenfg_b", "greenfg_bf"), 
			"[1;33":("yellowfg_b", "yellowfg_bf"), 
			"[1;34":("bluefg_b", "bluefg_bf"), 
			"[1;36":("cyanfg_b", "cyanfg_bf"), 
			"[30;43":("yellowbg_b", "yellowbg_bf"), 
			"[0":('std', "std_f"), "[":('std', "std_f")}
		markup = []
		st = ss.find("\x1b")
		if st == -1:
			return ss
			
		items = ss.split("\x1b")
		pt = 1
		if not ss.startswith("\x1b"):
			markup.append(items[0])
		
		for at in items[pt:]:
			if at == "[K":	# it...
				continue
			attr, text = at.split("m",1)
			if text != "":	# skip empty string
				markup.append((table[attr][invert], text))
			
		if len(markup) == 0:
			return ""
			
		return markup
		
	def genEdit(label, text, cbChange):
		w = urwid.Edit(label, text)
		urwid.connect_signal(w, 'change', cbChange)
		cbChange(w, text)
		#w = urwid.AttrWrap(w, 'edit')
		return w
		
	def genText(terminalText):
		line2 = Urwid.terminal2markup(terminalText)
		txt = urwid.Text(line2)
		#txt.origText = terminalText
		return txt
	
		
	def makeTextList(lstStr):
		outList = []
		for line in lstStr:
			outList.append(Urwid.genText(line))
		return outList
		
	def makeBtnList(lstStr, onClick, doApply=None):
		outList = []
		isFirst = True 
		for line in lstStr:
			if line.strip() == "":
				continue
				
			btn = Urwid.genBtn(line, onClick, isFirst, doApply)
			isFirst = False
			outList.append(btn)
		return outList
		
	def genBtn(terminalText, onClick, isFocus=False, doApply=None):
		text2 = Urwid.terminal2markup(terminalText, 1 if isFocus else 0)
		btn = Urwid.genBtnMarkup(text2, onClick, isFocus, doApply)
		btn.base_widget.origText = terminalText
		return btn
		
	def genBtnMarkup(markup, onClick, isFocus=False, doApply=None):
		btn = mButton(markup, onClick)
		#btn.origText = terminalText
		
		if doApply != None:
			doApply(btn)
			
		btn = urwid.AttrMap(btn, None, "reveal focus")
		return btn	
			
	def popupMsg(title, ss):
		def onCloseBtn(btn):
			g.loop.widget = g.mainLoop.widget.bottom_w
			
		txtMsg = urwid.Text(ss)
		btnClose = urwid.Button("Close", onCloseBtn)
		popup = urwid.LineBox(urwid.Pile([('pack', txtMsg), ('pack', btnClose)]), title)
		g.loop.widget = urwid.Overlay(urwid.Filler(popup), g.loop.widget, 'center', 20, 'middle', 10)
		
	def popupAsk(title, ss, onOk, onCancel = None):
		def onClickBtn(btn):
			if btn == btnYes:
				onOk()
			elif btn == btnNo:
				if onCancel != None: 
					onCancel()
					
			g.loop.widget = g.loop.widget.bottom_w
			
		txtMsg = urwid.Text(ss)
		btnYes = urwid.Button("Yes", onClickBtn)
		btnNo = urwid.Button("No", onClickBtn)
		popup = urwid.LineBox(urwid.Pile([('pack', txtMsg), ('pack', urwid.Columns([btnYes, btnNo]))]), title)
		g.loop.widget = urwid.Overlay(urwid.Filler(popup), g.loop.widget, 'center', 40, 'middle', 5)
		
	def popupAsk3(title, ss, btnName1, btnName2, btnName3, onBtn1, onBtn2, onBtn3 = None):
		def onClickBtn(btn):
			if btn == btnB1:
				onBtn1()
			elif btn == btnB2:
				onBtn2()
			elif btn == btnB3:
				if onBtn3 != None: 
					onBtn3()
					
			g.loop.widget = g.loop.widget.bottom_w
			
		txtMsg = urwid.Text(ss)
		btnB1 = urwid.Button(btnName1, onClickBtn)
		btnB2 = urwid.Button(btnName2, onClickBtn)
		btnB3 = urwid.Button(btnName3, onClickBtn)
		popup = urwid.LineBox(urwid.Pile([('pack', txtMsg), ('pack', urwid.Columns([btnB1, btnB2, btnB3]))]), title)
		g.loop.widget = urwid.Overlay(urwid.Filler(popup), g.loop.widget, 'center', 40, 'middle', 5)
		
	def popupInput(title, ss, onOk, onCancel = None):
		def onClickBtn(btn):
			if btn == btnOk:
				onOk(edInput.edit_text)
			elif btn == btnCancel:
				if onCancel != None: 
					onCancel()
					
			g.loop.widget = g.loop.widget.bottom_w
			
		edInput = urwid.Edit(ss)
		btnOk = urwid.Button("OK", onClickBtn)
		btnCancel = urwid.Button("Cancel", onClickBtn)
		popup = urwid.LineBox(urwid.Pile([('pack', txtMsg), ('pack', urwid.Columns([btnOk, btnCancel]))]), title)
		g.loop.widget = urwid.Overlay(urwid.Filler(popup), g.loop.widget, 'center', 40, 'middle', 5)


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

def urwidGitStatus():
	main = mDlgMainGitStatus()
	main.refreshFileList()
	if main.widgetFileList.itemCount == 0:
		print("No modified or untracked files")
		return
	
	g.dialog = main
	g.loop = urwid.MainLoop(main.mainWidget, g.palette, urwid.raw_display.Screen(),
		unhandled_input=urwidUnhandled, input_filter=urwidInputFilter)
	g.loop.run()
	
def cbWatchPipe(dlg, data):
	'''
	if g.sub.poll() != None:
		# cygwin에서 recvData받는중에 이게 참인 경우가 많다. - 리눅스는 바로 전달되서 이게 없다.
		self.headerText.set_text(self.header+"!!!")
		
	# cygwin에서는 발생 안함 - 리눅스도 안함
	if len(data) == 0:
		raise Exception("Crash len(data)==0!!!")
		self.headerText.set_text(self.header+"!!!")
		
		# eof
		if len(self.widgetFileList.body) == 0:
			self.widgetFileList.body.append(Urwid.genBtn("< No files >", self.cbFileSelect, True))
			return False
	'''			
			
	dlg.recvData(data)	
	
	
from distutils.spawn import find_executable
	
def urwidSubRun(dlg, doSubMake):
	g.dialog = dlg
	g.loop = urwid.MainLoop(dlg.mainWidget, g.palette, urwid.raw_display.Screen(),
		unhandled_input=urwidUnhandled, input_filter=urwidInputFilter)
		
	writeFd = g.loop.watch_pipe(lambda data: cbWatchPipe(dlg, data))
	g.sub = doSubMake(writeFd)
	g.loop.run()
		
def urwidFind(cmds):
	cmds[0] = find_executable(cmds[0])
	dlg = mDlgMainFind()
	urwidSubRun(dlg, lambda writeFd: subprocess.Popen(cmds, bufsize=0, stdout=writeFd, close_fds=True))
		
def urwidAck(cmds):
	cmds[0] = find_executable(cmds[0])
	dlg = mDlgMainAck()
	urwidSubRun(dlg, lambda writeFd: subprocess.Popen(cmds, bufsize=0, stdout=writeFd, close_fds=True))

		
def programPath(sub=None):
  pp = os.path.dirname(os.path.realpath(sys.argv[0]))
  if sub != None:
    pp = os.path.join(pp, sub)
  return pp

import datetime		

g = Global()
g.version = "1.0"
g._log = programPath("dc.log")
def logFunc(msg):
	timeStr = datetime.datetime.now().strftime("%m%d %H%M%S")
	with open(g._log, "a+", encoding="UTF-8") as fp:
		fp.write(timeStr + " " + msg + "\n")
	
g.log = logFunc

g.loop = None	# urwid

g.dialog = None


# (name, fg, bg, mono, fgHigh, bgHigh)
g.palette = [
		('std', 'light gray', 'black'),
		('std_f', 'black', 'dark cyan'),
		('reset', 'std'),
		("reset_f", "std_f"),
		('bold', 'light gray,bold', 'black'),
		('bold_f', 'light gray,bold', 'dark cyan'),
		('underline', 'light gray,underline', 'black'),
		('underline_f', 'light gray,underline', 'dark cyan'),

		('redfg', 'dark red', 'black'),
		('redfg_b', 'bold,dark red', 'black'),
		('redfg_f', 'light red', 'dark cyan'),
		('redfg_bf', 'bold,light red', 'dark cyan'),
		('greenfg', 'dark green', 'black'),
		('greenfg_b', 'bold,dark green', 'black'),
		('greenfg_f', 'light green', 'dark cyan'),
		('greenfg_bf', 'bold,light green', 'dark cyan'),
		('yellowfg', 'yellow', 'black'),
		('yellowfg_b', 'bold,yellow', 'black'),
		('yellowfg_f', 'yellow', 'dark cyan'),
		('yellowfg_bf', 'bold,yellow', 'dark cyan'),
		('bluefg', 'dark blue', 'black'),
		('bluefg_b', 'bold,dark blue', 'black'),
		('bluefg_f', 'light blue', 'dark cyan'),
		('bluefg_bf', 'bold,light blue', 'dark cyan'),
		('cyanfg', 'dark cyan', 'black'),
		('cyanfg_b', 'bold,dark cyan', 'black'),
		('cyanfg_f', 'light gray', 'dark cyan'),
		('cyanfg_bf', 'bold,light gray', 'dark cyan'),
		
		('redbg', 'black', 'dark red'),
		('yellowbg_b', 'black,bold', 'yellow'),
		('yellowbg_bf', 'black,bold', 'dark cyan'),	# it...

		('reveal focus', "black", "dark cyan", "standout"),
		]


def winTest():
	ss = system("c:\\cygwin64\\bin\\git.exe diff --color dc.py")

	kk = Urwid.terminal2markup(ss)
	st = ss.find("\x1b")
	print("%d %x %x %x %x" % (st, ss[0], ss[1], ss[2], ss[3]))
	sys.exit(0)

def getNonblocingInput():
	if select.select([sys.stdin], [], [], 0) == ([sys.stdin], [], []):
		return sys.stdin.read(255)

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

	sys.path.append(pp)
	m = __import__("path")
	g.lstPath = m.pathList
	
	if len(sys.argv) == 1:
		target = "st"
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
	elif target == "which":
		ss, status = systemSafe(" ".join(['"' + c + '"' for c in sys.argv[1:]]))
		print(ss)
		print("goto which path...")
		g.savePath(os.path.dirname(ss))
		return
	
	elif target == "find":
		# dc find . -name "*.py"

		#cmds = shlex.split(cmdLine)
		# find with shell=true not working on cygwin
		for idx,data in reversed(enumerate(sys.argv)):
			if data != "":
				sys.argv = sys.argv[:idx]
				break
				
		cmds = sys.argv[1:]
		urwidFind(cmds)
		return
		
	elif target == "findg":
		cmds = ["find", ".", "-name", sys.argv[2]]
		urwidFind(cmds)
		return
		
	elif target == "ack":
		# dc ack printf

		#cmds = shlex.split(cmdLine)
		# find with shell=true not working on cygwin
		for idx,data in reversed(list(enumerate(sys.argv))):
			if data != "":
				sys.argv = sys.argv[:idx+1]
				break
				
		cmds = sys.argv[1:]
		cmds.insert(1, "--group")
		cmds.insert(1, "--color")
		urwidAck(cmds)
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
	

