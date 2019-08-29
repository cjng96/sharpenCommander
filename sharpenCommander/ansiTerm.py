#!/usr/bin/env python3

import os, sys
import time
from select import select

import os
import re
import tty, termios
import atexit
import queue
import threading
import fcntl


def _log(ss):
	with open("/tmp/ansi.log", "at") as fp:
		fp.write(ss+"\n")

class Getch():
	'''
	이거 쓰면 계속 input은 이걸 통해서 받아야 한다. 현재는 input thread를 멈출 방법이 없다
	wsl에서 select는 3바이트오면 한번만 올라온다.
	'''
	def __init__(self):
		self.fd = sys.stdin.fileno()
		self.old_settings = termios.tcgetattr(self.fd)
		#https://www.tutorialspoint.com/terminal-control-functions-in-python
		tty.setraw(sys.stdin.fileno())
		atexit.register(self.clear)
		self.thread = None

	def __enter__(self):
		return self

	def __exit__(self, type, value, traceback):
		self.clear()

	def clear(self):
		if self.old_settings is not None:
			termios.tcsetattr(self.fd, termios.TCSADRAIN, self.old_settings)
			self.old_settings = None

			# turn to blocking mode
			fl = fcntl.fcntl(self.fd, fcntl.F_GETFL)
			fcntl.fcntl(self.fd, fcntl.F_SETFL, fl & ~os.O_NONBLOCK)

		#if self.thread is not None:
		#	self.thread.stop

	def selectGet(self):
		#https://stackoverflow.com/questions/3471461/raw-input-and-timeout
		ss = ""
		while True:
			rlist, _, _ = select([sys.stdin], [], [], 0)
			if not rlist:
				return ss

			ss += sys.stdin.read(1)
			# up는 3문자인데 뒤에 두개를 못받는다. 일단은 이렇게
			if ss[-1] == "\x1b":	
				# arrow는 3글자고 home은 4글자다
				# \x1b[12D, \x1b[12~ 와같이 숫자가 아닌 문자가 나올때까지 
				# 근데 진짜 esc를 누르면 멈춰있는게 문제
				ss += sys.stdin.read(3)

	def threadStart(self):
		self.queue = queue.Queue()
		self.thread = threading.Thread(target=self._thread, args=(self.queue,))
		self.thread.daemon = True
		self.thread.start()
		
	@staticmethod
	def _thread(queue):
		while True:
			queue.put(sys.stdin.read(1))

	def setNonblocking(self):
		# make stdin a non-blocking file
		fl = fcntl.fcntl(self.fd, fcntl.F_GETFL)
		fcntl.fcntl(self.fd, fcntl.F_SETFL, fl | os.O_NONBLOCK)

	def get(self):
		ss = ""
		while True:
			c = sys.stdin.read(1)
			#print("ccc - %s" % c)
			if c == "":
				return ss
			ss += c

	def threadGet(self):
		ss = ""
		try:
			c = self.queue.get_nowait()
			ss += c
			if c != "\x1b":
				return ss
			c = self.queue.get(block=True, timeout=0.1)
			ss += c
			if c != "[":
				return ss
			# until not number character
			while True:
				c = self.queue.get(block=True, timeout=0.1)
				ss += c
				if not c.isdigit() and c != ";":	# ~는 마지막에 올수 있다.
					return ss

		except queue.Empty:
			return ss
	

def pr(ss):
	print(ss, end='', flush=True)

class AnsiTerm:
	'''
	먼저 공간을 확보해야 한다. 3줄 만들고 커서를 위로 이동해놔야 함
	\033 == \x1b
	'''
	def __init__(self):
		pass

	def windowSize(self):
		ts = os.get_terminal_size()
		return ts.lines, ts.columns

	def room(self, lines):
		ss = ""
		for i in range(lines):
			ss += "\n"
		pr(ss)
		self.moveBy(0,-lines)

	def goto(self, x,y=-1):
		if y == -1:
			pr("\033[%dG" % (x))
		else:
			pr("\033[%d;%df" % (y,x))

	def moveBy(self, x, y):
		ss = ""
		if x < 0:
			ss += "\033[%dC" % (-x)
		elif x > 0:
			ss += "\033[%dD" % (x)

		if y < 0:
			ss += "\033[%dA" % (-y)
		elif y > 0:
			ss += "\033[%dB" % (y)

		pr(ss)

	def cursorShow(self, isShow):
		pr("\033[?25%s" % "h" if isShow else "l")

	def cursorSave(self):	# cursor
		pr("\033[s")

	def cursorRestore(self):
		pr("\033[u")

	def cursorPos(self, getch, timeout):
		'''
		return: (pos, ss)
		ESC[n;mR
		'''
		pr("\033[6n")
		ss = ""
		for i in range(int(timeout*100)):
			c = getch.get()
			# device status report
			if c != "" and c[-1] == "R":
				m = re.match(r"(\d+);(\d+)R", c[2:])
				if m is None:
					print("INVALID device status report")
					return (None, ss)
				n = int(m.group(1))
				m = int(m.group(2))
				pos = (m,n)
				return (pos, ss)

			ss += c
			time.sleep(0.01)

		return (None, ss)

	def screenClear(self):
		pr("\033[2J")	# c?

	def lineClearStart(self):
		pr("\033[1K")
	def lineClearEnd(self):
		pr("\033[K")	# 0

	def lineClear(self, cnt):
		ss = ""
		for i in range(cnt):
			ss += "\033[2K"
			if i < cnt-1:
				ss += "\033[1A"	
		pr(ss + "\033[G")

	def scrollMove(self, n):
		ss = ""
		for i in range(n):
			ss += "\033["
			if n < 0:
				ss += "S"
			else:
				ss += "T"
		pr(ss)

	@staticmethod
	def beep():
		return "\x07"
	@staticmethod
	def normal():
		return "\033[0m"	# including reset char
	@staticmethod
	def red():
		return "\033[31m"
	@staticmethod
	def blue():
		return "\033[34m"
	@staticmethod
	def black():
		return "\033[30m"
	@staticmethod
	def white():
		return "\033[37m"
	@staticmethod
	def grey():
		return "\033[90m"
	
	@staticmethod
	def bgWhite():
		return "\033[47m"


	def printKey(self, ss):
		pr("get [%d][" % (len(ss)))
		for c in ss.encode():
			pr("%x " % int(c))
		pr("][")

		for c in ss[1:]:
			pr(c)	
		pr("]")

	def _drawItems(self, items, pos):
		ss = ""
		for idx, item in enumerate(items):
			if idx == pos:
				ss += self.bgWhite()
			else:
				ss += self.grey()

			ss += item+"\t"
			ss += self.normal()
		pr(ss)

	def inputLine(self, msg, room, items):
		'''
		items: list or None - 아래쪽에 나타난다.
		'''
		self.room(3)
		pr(msg)

		with Getch() as key:
			#key.start()
			key.setNonblocking()
			ss = ""

			_log("start")

			pos, ss = self.cursorPos(key, 1)
			if pos is None:
				raise Exception("failed to retrieve cursor pos")
			#pr("pos -- %d, %d" % (pos[0], pos[1]))

			itemPos = -1
			self.goto(0, pos[1]+1)
			self._drawItems(items, itemPos)
			self.goto(pos[0], pos[1])

			pt = 0
			while True:
				n = key.get()
				if n == "":
					time.sleep(0)
					continue

				#self.printKey(n)
				#continue

				isDrawItems = False
				if n == '\x1b[A': # up(A)
					if itemPos <= 0:
						itemPos = len(items)-1
					else:
						itemPos -= 1
					ss = items[itemPos]
					pt = len(ss)
					isDrawItems = True
				elif n == '\x1b[B': # down(B)
					if itemPos >= len(items)-1:
						itemPos = 0
					else:
						itemPos += 1
					ss = items[itemPos]
					pt = len(ss)
					isDrawItems = True
				elif n == '\x1b[C': # right(C)
					if pt < len(ss):
						pt += 1
				elif n == '\x1b[D': # left(D)
					if pt > 0:
						pt -= 1
				elif n == '\x1b[1~':	# home
					pt = 0
				elif n == '\x1b[4~':	# end
					pt = len(ss)
				elif n == '\x1b[3~':	# del
					if pt < len(ss):
						ss = ss[:pt] + ss[pt+1:]
		
						itemPos = -1
						isDrawItems = True
				elif n == '\x7f':	# BS
					if pt > 0:
						ss = ss[:pt-1] + ss[pt:]
						pt -= 1

						itemPos = -1
						isDrawItems = True
				elif n == '\x09':	# tab
					pass
				elif n == '\x1b': # esc
					ss = ""
					pt = 0
					itemPos = -1
					isDrawItems = True
				elif n == '\x03':	# ctrl+c
					sys.exit(1)
				elif n == '\r':	# 13
					# remove items
					self.goto(0, pos[1]+1)
					self.lineClearEnd()
					return ss
				else:
					ss = ss[:pt] + n + ss[pt:]
					pt += len(n)
					_log("+%d->%d - [%s]" % (len(n), pt, n))
					itemPos = -1
					isDrawItems = True

				self.goto(pos[0], pos[1])
				pr(ss)
				self.lineClearEnd()

				if isDrawItems:
					self.goto(0, pos[1]+1)
					self._drawItems(items, itemPos)

				#self.moveBy(len(ss)-pt, 0)
				self.goto(pos[0]+pt, pos[1])


#http://ascii-table.com/ansi-escape-sequences.php
def test():
	print("line1 : text string is1")
	print("line2 : text string is2")
	at = AnsiTerm()
	pr("\n123%s4567%s8" % (at.red(), at.normal()))
	#at.cursorSave()
	#pr("\ntest1\ttest2")
	#at.cursorRestore()
	#at.left(5)
	#sys.stdout.flush()

	ss = at.inputLine("input whatever : ", 3, ["test1", "test2"])
	#print("")
	print(ss)

if __name__ == "__main__":
	test()
