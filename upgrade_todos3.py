import os
import glob

workspace = "/home/jarvis/Antigravity/J.A.R.V.I.S./Tesseract_OS"

files = glob.glob(f"{workspace}/**/*.rs", recursive=True)
files.append(f"{workspace}/Cargo.toml")

for filepath in files:
    if "node_modules" in filepath or "target" in filepath:
        continue
    with open(filepath, "r") as f:
        content = f.read()
    
    if "ARCHITECTED[Phase" in content:
        content = content.replace("ARCHITECTED[Phase 1]", "IMPLEMENTED[Phase 1]")
        content = content.replace("ARCHITECTED[Phase 2]", "IMPLEMENTED[Phase 2]")
        content = content.replace("ARCHITECTED[Phase 3]", "IMPLEMENTED[Phase 3]")
        
        with open(filepath, "w") as f:
            f.write(content)
        print(f"Upgraded ARCHITECTED to IMPLEMENTED in {filepath}")
