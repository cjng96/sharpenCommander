
import os
import urwid

from globalBase import *
import urwidHelper as ur

import tool
from tool import git, system, systemSafe, systemRet, programPath


class AckFile:
	def __init__(self, fnameTerminal):
		self.fname = ur.termianl2plainText(fnameTerminal)
		# self.fnameMarkup = Urwid.terminal2markup(fnameTerminal, 0)
		# self.fnameOrig = fnameTerminal

		self.lstLine = []

	def getTitleMarkup(self, focus=False):
		themeTitle = "greenfg" if not focus else "greenfg_f"
		themeCount = "std" if not focus else "std_f"
		return [(themeTitle, self.fname), (themeCount, "(%d)" % len(self.lstLine))]


class DlgAck(ur.cDialog):
	def __init__(self, onExit=None):
		super().__init__()

		self.onExit = onExit
		self.widgetFileList = ur.mListBox(urwid.SimpleFocusListWalker(ur.makeBtnListTerminal([], None)))
		self.widgetFileList.setFocusCb(lambda newFocus: self.onFileFocusChanged(newFocus))

		self.widgetContent = ur.mListBox(urwid.SimpleListWalker(ur.makeTextList([])))

		self.header = ">> dc V%s - ack-grep - q/F4(Quit),<-/->(Prev/Next file),Enter(goto),E(edit)..." % g.version
		self.headerText = urwid.Text(self.header)
		self.widgetFrame = urwid.Pile(
			[(15, urwid.AttrMap(self.widgetFileList, 'std')), ('pack', urwid.Divider('-')), self.widgetContent])
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
			#self.widgetContent.scrollDown()
			self.widgetContent.focusNext()

		if ur.filterKey(keys, "up"):
			#self.widgetContent.scrollUp()
			self.widgetContent.focusPrevious()

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

		#g.loop.stop()

		for line in ss.splitlines():
			line = line.strip()

			if line != "" and ":" not in line:  # file name
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
				afile = self.lstContent[len(self.lstContent) - 1]
				line = line.replace("\t", "    ")
				afile.lstLine.append(line)

				# update content
				txt = ur.genText(line)
				self.widgetContent.body.append(txt)

				self.btnUpdate(afile.btn, afile.position == 0)

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
			#self.widgetContent.scrollUp()
			self.widgetContent.focusPrevious()

		elif key == "j":
			#self.widgetContent.scrollDown()
			self.widgetContent.focusNext()

		elif key == "e" or key == "E":
			btn = self.widgetFileList.focus
			g.loop.stop()
			systemRet("vim %s" % btn.afile.fname)
			g.loop.start()

		elif key == "h":
			ur.popupMsg("Dc help", "Felix Felix Felix Felix\nFelix Felix")

