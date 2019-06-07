from setuptools import setup, find_packages
import os
import re
import site
import sys
from os.path import expanduser
from setuptools.command.install import install
from setuptools.command.develop import develop

import sharpenCommander



setup(
	name             = 'sharpen-commander',
	version          = sharpenCommander.__version__,
	description      = 'Console based Moving to folders. GIT repo management tool.',
	long_description = open('README.md').read(),
	long_description_content_type='text/markdown',
	author           = 'Felix Choi',
	author_email     = 'cjng96@gmail.com',
	url              = 'https://github.com/cjng96/sharpenCommander',
	license          = "LGPL",
	#download_url     = 'https://github.com/cjng96/sharpenCommander/archive/0.1.tar.gz',
	install_requires = ["click", "urwid", "PyYAML"],
	packages         = find_packages(exclude = ['docs', 'tests*']),
	package_data     = {'sharpenCommander': ['script-*.sh', 'virenv-*.sh']},
	include_package_data=True,	
	keywords         = ['git management', 'folder management'],
	python_requires  = '>=3',
	platforms        = "Posix; MacOS X; Windows",
	zip_safe         = False,
	entry_points     = {"console_scripts": ["sc=sharpenCommander:run.run"]},
	classifiers      = [
		"Operating System :: OS Independent",		
		'Programming Language :: Python',
		'Programming Language :: Python :: 3',
		'Programming Language :: Python :: 3.2',
		'Programming Language :: Python :: 3.3',
		'Programming Language :: Python :: 3.4',
		'Programming Language :: Python :: 3.5',
		"Programming Language :: Python :: 3.6",
		"Programming Language :: Python :: 3.7",
		"Programming Language :: Python :: 3.8",
	]
)
