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
    
    if "HORIZON[P2]" in content or "HORIZON[P3]" in content or "TODO[P1]" in content:
        content = content.replace("HORIZON[P2]", "TODO[Phase 2]")
        content = content.replace("HORIZON[P3]", "TODO[Phase 3]")
        content = content.replace("TODO[P1]", "TODO[Phase 1]")
        
        with open(filepath, "w") as f:
            f.write(content)
        print(f"Upgraded TODO flags in {filepath}")

