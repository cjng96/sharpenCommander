# coding: utf-8

import datetime

#g.logPath = programPath("dc.log")
class GlobalBase(object):
	def __init__(self):
		self.app = None

	def __getattr__(self, name):
		return getattr(self.app, name)


class Program(object):
	def __init__(self, verStr, logPath):
		self.logPath = logPath
		self.version = verStr

	def log(self, lv, msg):
		timeStr = datetime.datetime.now().strftime("%m%d %H%M%S")
		with open(g.logPath, "a+", encoding="UTF-8") as fp:
			fp.write("%s [%d] %s\n" % (timeStr, lv, msg))


# should assign it
g = GlobalBase()



