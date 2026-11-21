"""Where cargo writes binaries, which a worktree may share with the main checkout (tools/share-target.sh)."""
import json
import subprocess
from pathlib import Path


def target_dir(root):
    meta = subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps"], cwd=root, check=True, capture_output=True, text=True)
    return Path(json.loads(meta.stdout)["target_directory"])
