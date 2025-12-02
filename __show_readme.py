from pathlib import Path
text = Path("README.md").read_text(encoding="utf-8")
start = text.index("## dYZr Kullan")
end = text.index("## dY\"\x15 Geli")
print(repr(text[start:end]))
