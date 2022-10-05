import shutil

shutil.rmtree("app", ignore_errors=True)
shutil.copytree("gui/public", "app", dirs_exist_ok=True)
