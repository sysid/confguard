import os
import subprocess
import shutil
from pathlib import Path

ROOT_DIR = Path(__file__).parent.parent

file_path = os.getenv("CONFGUARD_BASE_DIR")

if not file_path:
    raise ValueError("Environment variable CONFGUARD_BASE_DIR is not set.")

if "sec-sops" in file_path:
    raise ValueError("Environment variable CONFGUARD_BASE_DIR must not contain sec-sops")

# Get the confguard_base path
confguard_base_dir = file_path

# Ensure the directory exists
if not os.path.isdir(confguard_base_dir):
    Path(confguard_base_dir).mkdir(parents=True, exist_ok=True)

# Clear the directory contents
shutil.rmtree(confguard_base_dir)
Path(confguard_base_dir).mkdir(parents=True, exist_ok=True)
shutil.copytree(ROOT_DIR / "confguard/tests/resources/data/testprj", Path(confguard_base_dir) / "testprj")

result = subprocess.run(["tree", "-a", confguard_base_dir], capture_output=True, text=True)
print(result.stdout)
