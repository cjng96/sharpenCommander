
import os
import urwid

from globalBase import *

import urwidHelper as ur
import tool

#import dc
import myutil


class mDlgMainFind(ur.cDialog):
	def __init__(self, onExit=None):
		super().__init__()

		self.onExit = onExit
		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([], None)))
		self.widgetFileList.setFocusCb(lambda newFocus: self.onFileFocusChanged(newFocus))
		self.widgetContent = ur.mListBox(urwid.SimpleListWalker(ur.makeTextList(["< Nothing to display >"])))
		self.widgetContent.isViewContent = True

		self.header = ">> dc V%s - find - q/F4(Quit),<-/->(Prev/Next file),Enter(goto),E(edit)..." % g.version
		self.headerText = urwid.Text(self.header)
		self.widgetFrame = urwid.Pile(
			[(15, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
		self.mainWidget = urwid.Frame(self.widgetFrame, header=self.headerText)

		self.cbFileSelect = lambda btn: self.onFileSelected(btn)
		self.content = ""
		self.selectFileName = ""

	def onFileFocusChanged(self, newFocus):
		# old widget
		# widget = self.widgetFileList.focus
		# markup = ("std", widget.base_widget.origTxt)
		# widget.base_widget.set_label(markup)

		# widget = self.widgetFileList.body[newFocus]
		# markup = ("std_f", widget.base_widget.origTxt)
		# widget.base_widget.set_label(markup)
		widget = self.widgetFileList.body[newFocus]

		self.widgetFileList.set_focus_valign("middle")

		self.selectFileName = myutil.gitFileBtnName(widget)

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
		self.selectFileName = myutil.gitFileBtnName(btn)
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

			# markup = ur.terminal2markup(line, 0)
			# markupF = ur.terminal2markup(line, 1)
			markup = ("std", line)
			markupF = ('std_f', line)

			btn = ur.genBtn(markup, markupF, self.cbFileSelect, len(self.widgetFileList.body) == 0)
			self.widgetFileList.body.append(btn)
			if len(self.widgetFileList.body) == 1:
				self.onFileFocusChanged(0)

		return True

	def unhandled(self, key):
		if key == 'f4' or key == "q":
			#raise urwid.ExitMainLoop()
			self.close()

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
			fname = myutil.gitFileBtnName(btn)

			g.loop.stop()
			tool.systemRet("vim %s" % fname)
			g.loop.start()

		elif key == "h":
			ur.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")
