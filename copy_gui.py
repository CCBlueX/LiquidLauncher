import shutil

shutil.rmtree("app", ignore_errors=True)
shutil.copytree("app/public", "app", dirs_exist_ok=True)
