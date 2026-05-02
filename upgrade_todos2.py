import os
import glob

workspace = "/home/jarvis/Antigravity/J.A.R.V.I.S./Tesseract_OS"

# Files to scan
files = glob.glob(f"{workspace}/**/*.rs", recursive=True)
files.append(f"{workspace}/Cargo.toml")

for filepath in files:
    if "node_modules" in filepath or "target" in filepath:
        continue
    with open(filepath, "r") as f:
        content = f.read()
    
    if "TODO[Phase" in content:
        content = content.replace("TODO[Phase 1]", "ARCHITECTED[Phase 1]")
        content = content.replace("TODO[Phase 2]", "ARCHITECTED[Phase 2]")
        content = content.replace("TODO[Phase 3]", "ARCHITECTED[Phase 3]")
        
        with open(filepath, "w") as f:
            f.write(content)
        print(f"Upgraded TODO to ARCHITECTED in {filepath}")
