import shutil

shutil.rmtree("run/app", ignore_errors=True)
shutil.copytree("gui/public", "run/app", dirs_exist_ok=True)
