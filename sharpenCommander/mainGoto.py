
import os
import urwid
import subprocess
import time

from multiprocessing import Pool

from .urwidHelper import *
from .tool import *
from .myutil import *

from .globalBase import *
from .mainRegList import DlgRegFolderSetting


def repoGetStatus(item):
	status  = dict(M=0, E=None)
	if not item["repo"]:
		return status

	try:
		ss = system("git status -s")
		if ss != "":
			status["M"] = 1
	except subprocess.CalledProcessError as e:
		status["E"] = str(e)

	return status

gGoto = None

class mDlgGoto(cDialog):
	def __init__(self, onExit):
		super().__init__()

		global gGoto
		gGoto = self

		self.it = None
		self.allDirs = []

		self.editInCode = False

		self.onExit = onExit
		self.widgetFileList = mListBox(urwid.SimpleFocusListWalker(btnListMakeTerminal([], None)))
		#self.widgetFileList.setFocusCb(lambda newFocus: self.onFileFocusChanged(newFocus))
		#self.widgetContent = mListBox(urwid.SimpleListWalker(makeTextList(["< Nothing to display >"])))
		#self.widgetContent.isViewContent = True

		self.header = ">> dc V%s - folder list - %%d - JK(move), E(modify), del" % g.version
		self.headerText = urwid.Text(self.header)
		self.curPathText = urwid.Text("Current path : " + os.getcwd())

		#self.widgetFrame = urwid.Pile(
		#	[(15, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.widgetFrame = urwid.AttrMap(self.widgetFileList, 'std')
		self.edInput = editGen("$ ", "", lambda edit, text: self.onInputChanged(edit, text))
		content = urwid.Pile([("pack", self.curPathText),self.widgetFrame])
		self.mainWidget = urwid.Frame(content, header=self.headerText, footer=self.edInput)

		#self.itemList = None
		#self.cbFileSelect = lambda btn: self.onFileSelected(btn)

		self.mainWidget.set_focus("footer")

	def init(self):
		self.refreshFile()
		return True

	def onInputChanged(self, edit, text):
		if gGoto.editInCode: 
			return

		last = ""
		if len(text) > 0:
			last = text[-1]
		#dlog("text - %s - %s" % (text, self.editInCode))
		if last in ["E", 'J', 'K', "H", 'D', 'Q', "P", "U"]:
			def _cb(self, data):
				#data["dlg"].edInput.set_edit_text(data["text"][:-1])
				gGoto.editInCode = True
				try:
					edit.set_edit_text(data["text"][:-1])
				finally:
					gGoto.editInCode = False

			g.loop.set_alarm_in(0.00001, _cb, dict(dlg=self, edit=edit, text=text))
			self.unhandled(last)
			#traceback.print_stack()
			return

		self.refreshFile(text)

	def onFileSelected(self, btn):
		if btn.attr is None:
			return

		pp = btn.attr["path"]
		os.chdir(pp)
		self.close()

	def refreshFile(self, filterStr=None):
		#oldPath = os.getcwd()
		if filterStr is None:
			filterStr = self.edInput.get_edit_text()

		if filterStr.find("/") == -1:
			if filterStr != "":
				lst = g.regFindItems(filterStr)
			else:
				lst = g.regList[:]
		else:
			lst = []

		resultList = []
		for x in lst:
			resultList.append(("greenfg", x["path"], x))

		# local dirs
		#if filterStr.find(" ") != -1:
		cwd = os.getcwd()
		if self.it is None:
			self.it = myWalk(cwd, [".git", "node_modules", ".pnpm-store"])
			self.allDirs = []

			# 일단은 100개만
			for i in range(100):
				try:
					pp, dirs, files = next(self.it)
				except StopIteration:
					break

				for dir in dirs:
					self.allDirs.append(os.path.join(pp, dir)[len(cwd)+1:])

				#dlog(pp)
			#dlog("%d count " % len(self.allDirs))

		lst = []
		filterList = filterStr.lower().split(" ")
		for dir in self.allDirs:
			pt = 0
			d2 = dir.lower()
			for ff in filterList:
				pt2 = d2.find(ff, pt)
				if pt2 == -1:
					pt = -1
					break
				pt = pt2+len(ff)

			if pt != -1:
				lst.append(dict(names=[dir], path=dir))
		dlog("lst cnt : %d" % len(lst))
		for x in lst:
			resultList.append(("cyanfg", x["path"], x))

		ss = self.header % len(resultList)
		self.headerText.set_text(ss)

		idx = 0
		#if self.widgetFileList.body.focus is not None:
		#	idx = self.widgetFileList.body.focus

		refreshBtnListMarkupTuple(resultList, self.widgetFileList, lambda btn: self.onFileSelected(btn))

		#if idx >= len(self.widgetFileList.body):
		#	idx = len(self.widgetFileList.body)-1
		self.widgetFileList.set_focus(idx)

	def unhandled(self, key):
		#dlog("key - %s" % key)
		if key == 'f4' or key == "Q" or key == "esc":
			self.close()
		elif key == "H" or key == "enter":
			self.onFileSelected(self.widgetFileList.body.get_focus()[0].original_widget)

		elif key == "J":  # we can't use ctrl+j since it's terminal key for enter replacement
			self.widgetFileList.focusNext()
		elif key == "K":
			self.widgetFileList.focusPrevious()

		elif key == "up":
			self.widgetFileList.focusPrevious()
		elif key == "down":
			self.widgetFileList.focusNext()

		elif key == "esc":
			self.edInput.set_edit_text("")
			self.refreshFile()

		elif key == "U":
			os.chdir(os.path.dirname(os.getcwd()))
			self.curPathText.set_text("Current path : " + os.getcwd())
			self.it = None
			self.allDirs = []
			#self.refreshFile()
			self.edInput.set_edit_text("")

		elif key == "E":
			item = self.widgetFileList.focus
			target = item.original_widget.attr
			if not target.starswith("/"):
				return
			self.doEdit(item.original_widget.attr)
			self.refreshFile()

		elif key == "D" or key == "delete":
			target = self.widgetFileList.focus.original_widget.attr
			if not target.starswith("/"):
				return
			g.regRemove(target["path"])
			self.refreshFile()

		elif key == "P":
			# 모든 repo udpate
			g.loop.stop()

			oldPath = os.getcwd()
			cnt = len(self.widgetFileList.body)
			for idx, item in enumerate(self.widgetFileList.body):
				attr = item.original_widget.attr
				pp = attr["path"]
				#os.chdir(pp)

				repoStatus = attr["repoStatus"]
				if attr["repo"]:
					if "M" in repoStatus:
						isModified = repoStatus["M"]
						try:
							print("[%d/%d] - %s" % (idx + 1, cnt, pp))
							if isModified:
								print("  git fetch")
								system("cd '%s'; git fetch" % pp)
								# 수정내역이 있으면 어차피 최신으로 못만든다.
							else:
								print("  git pull -r")

								# TODO: no has tracking branch
								system("cd '%s'; git pull -r" % pp)
						except subprocess.CalledProcessError as e:
							repoStatus["E"] = e

			os.chdir(oldPath)
			input("Enter to return...")
			g.loop.start()

	def doEdit(self, item):
		def onExit():
			g.doSetMain(self)

		dlg = DlgRegFolderSetting(onExit, item)
		g.doSetMain(dlg)