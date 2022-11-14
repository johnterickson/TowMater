# Packages the bot up into a zip file for submission to tournaments.
#
# What this script does: does a Rust release build, copies the new binary and
# files in rustbot_dev into a new folder, tweaks the bot config, and zips it up.

import glob
import os
import re
import shutil
import sys

# Assume the name of the bot is the name of the current folder
bot_name = os.path.basename(os.getcwd())

# Do a Rust release build
build_return_code = os.system('cargo b --release')
if build_return_code > 0:
    print('Build failed!')
    exit(1)

# Make a new folder
FOLDER_NAME = 'rustbot'
if not os.path.exists(FOLDER_NAME):
    os.mkdir(FOLDER_NAME)

# Find the new binary
binaries = glob.glob('target/release/*.exe')
if not binaries:
    print('No .exe files found in target/release!')
    exit(1)
if len(binaries) > 1:
    print('Multiple .exe files found in target/release - not sure which one to package!')
    exit(1)
binary = binaries[0]

# Copy the binary into the folder
exe_name = bot_name + '.exe'
shutil.copyfile(binary, os.path.join(FOLDER_NAME, exe_name))

# Copy the configuration into the folder
for each_file in ('appearance.cfg', 'rustbot.cfg', 'rustbot.py'):
    shutil.copy(os.path.join('rustbot_dev', each_file), FOLDER_NAME)


# Make the bot config point to the new location of the executable
config_path = os.path.join(FOLDER_NAME, 'rustbot.cfg')
with open(config_path, 'r') as config_file:
    config_file_text = config_file.read()

config_file_text = re.sub(r'path = .*\n', 'path = ' + exe_name + '\n', config_file_text)

with open(config_path, 'w') as config_file:
    config_file.write(config_file_text)

# Make the zip file
shutil.make_archive(bot_name, 'zip', FOLDER_NAME)

# Remove the folder
shutil.rmtree(FOLDER_NAME)
