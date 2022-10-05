import shutil
import os

if os.path.exists("run/app"):
    os.rmdir("run/app")
shutil.copytree("gui/public", "run/app", dirs_exist_ok=True)
