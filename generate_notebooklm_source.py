import os
import glob
import subprocess

# Files to export for NotebookLM
# We target all the cryptic md files, the main documentation, and some core config.
target_patterns = [
    "!\[(*\]*.md",   # the encrypted/ontological messages
    "*.txt",         # text files like manifestos, proofs of love
    "*.md",          # other markdown files like PHILOSOPHY, INCEPTION, etc.
    "Cargo.toml",    # system architecture definitions
]

output_file = "TESSERACT_NOTEBOOKLM_EXPORT.md"

def generate_export():
    print("Generating NotebookLM export...")
    files_to_export = []
    
    # Collect files
    for pattern in target_patterns:
        files_to_export.extend(glob.glob(pattern))
    
    # Remove duplicates and the output file itself if it exists
    files_to_export = list(set(files_to_export))
    if output_file in files_to_export:
        files_to_export.remove(output_file)

    # Filter to git-tracked files only — prevents accidentally exporting
    # gitignored files (e.g. local secrets, scratch notes) that happen
    # to match the glob patterns above.
    # Use -z to avoid octal-escaping of non-ASCII filenames (e.g. ŧøß),
    # which would otherwise break the set membership check against glob()
    # results that come back in raw UTF-8.
    tracked = set(
        subprocess.run(
            ['git', 'ls-files', '-z'],
            capture_output=True, check=True
        ).stdout.decode('utf-8').split('\0')
    )
    tracked.discard('')  # split leaves a trailing empty string
    files_to_export = [f for f in files_to_export if f in tracked]

    # Sort files alphabetically for consistency
    files_to_export.sort()
    
    with open(output_file, 'w', encoding='utf-8') as outfile:
        outfile.write("# TESSERACT_OS : KNOWLEDGE BASE EXPORT FOR NOTEBOOKLM\n\n")
        outfile.write("This document is a compiled snapshot of the Tesseract OS repository. It contains the ontological, technical, and cryptographic memory of the system, specifically designed to be ingested by an LLM.\n\n")
        outfile.write("---\n\n")
        
        for file_path in files_to_export:
            # Skip some large irrelevant files or non-text files if accidentally caught
            if os.path.getsize(file_path) > 500000: # skip files > 500KB
                continue
                
            try:
                with open(file_path, 'r', encoding='utf-8') as infile:
                    content = infile.read()
                    outfile.write(f"## FILE: {file_path}\n\n")
                    outfile.write("```text\n")
                    outfile.write(content)
                    if not content.endswith('\n'):
                        outfile.write("\n")
                    outfile.write("```\n\n")
                    outfile.write("---\n\n")
            except Exception as e:
                print(f"Skipping {file_path} due to read error: {e}")
                
    print(f"Export complete! File saved as {output_file}")
    print(f"Total files exported: {len(files_to_export)}")

if __name__ == "__main__":
    generate_export()
