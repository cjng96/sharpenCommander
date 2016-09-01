# coding: utf-8

import datetime



class Err(Exception):
	def __init__(self, msg):
		super().__init__(msg)

# FileNotFoundError(errno.ENOENT, os.strerror(errno.ENOENT), filename)
class ErrNoExist(Err):
	def __init__(self, msg, path):
		super().__init__(msg)
		self.path = path

# without callstack
class ErrFailure(Err):
	def __init__(self, msg):
		super().__init__(msg)



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
		g.app = self

	def log(self, lv, msg):
		timeStr = datetime.datetime.now().strftime("%m%d %H%M%S")
		with open(g.logPath, "a+", encoding="UTF-8") as fp:
			fp.write("%s [%d] %s\n" % (timeStr, lv, msg))


# should assign it
g = GlobalBase()



