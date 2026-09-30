#!/usr/bin/env python3
"""Collects every translatable string and updates the gettext catalogs.

Usage: python tools/i18n.py
Sources: @tr("...") in crates/gui/ui/*.slint, tr("...") / trf!("...") in crates/gui/src/*.rs,
and the EXTRA_TEXTS list in crates/gui/src/i18n.rs.
Writes crates/gui/lang/dpimech-gui.pot and updates crates/gui/lang/<lang>/LC_MESSAGES/dpimech-gui.po:
existing translations are kept, new strings are added untranslated, unused ones are dropped.
Translators only edit the .po files (Poedit, or any text editor).
"""

import glob
import os
import re

ROOT = os.path.join(os.path.dirname(__file__), "..")
GUI = os.path.join(ROOT, "crates", "gui")
LANG = os.path.join(GUI, "lang")
DOMAIN = "dpimech-gui"
STRING = r'"((?:[^"\\]|\\.)*)"'


def unescape(s):
    return re.sub(r"\\(.)", lambda m: {"n": "\n", "t": "\t"}.get(m.group(1), m.group(1)), s)


def collect():
    found = {}  # msgid -> first location

    def add(text, where):
        text = unescape(text)
        if text and text not in found:
            found[text] = where

    for path in sorted(glob.glob(os.path.join(GUI, "ui", "*.slint"))):
        src = open(path, encoding="utf-8").read()
        for m in re.finditer(r"@tr\(\s*" + STRING, src):
            add(m.group(1), os.path.relpath(path, GUI) + ":" + str(src.count("\n", 0, m.start()) + 1))
    for path in sorted(glob.glob(os.path.join(GUI, "src", "*.rs"))):
        src = open(path, encoding="utf-8").read()
        for m in re.finditer(r"(?<![\w.])(?:tr\(|trf!\()\s*" + STRING, src):
            line_start = src.rfind("\n", 0, m.start()) + 1
            if src[line_start:m.start()].lstrip().startswith("//"):
                continue  # examples in comments
            add(m.group(1), os.path.relpath(path, GUI) + ":" + str(src.count("\n", 0, m.start()) + 1))
        extra = re.search(r"EXTRA_TEXTS: &\[&str\] = &\[(.*?)\];", src, re.S)
        if extra:
            for m in re.finditer(STRING, extra.group(1)):
                add(m.group(1), os.path.relpath(path, GUI) + " (EXTRA_TEXTS)")
    return found


def quote(s):
    s = s.replace("\\", "\\\\").replace('"', '\\"').replace("\t", "\\t")
    if "\n" in s.rstrip("\n"):
        parts = s.split("\n")
        lines = [p + "\\n" for p in parts[:-1]] + ([parts[-1]] if parts[-1] else [])
        return '""\n' + "\n".join('"' + l + '"' for l in lines)
    return '"' + s.replace("\n", "\\n") + '"'


def read_po(path):
    entries, msgid, msgstr, field = {}, None, None, None
    header = ""
    if not os.path.exists(path):
        return header, entries
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if line.startswith("msgid "):
            if msgid is not None:
                entries[msgid] = msgstr
            msgid, msgstr, field = unescape(line[6:].strip()[1:-1]), "", "id"
        elif line.startswith("msgstr "):
            msgstr, field = unescape(line[7:].strip()[1:-1]), "str"
        elif line.startswith('"'):
            if field == "id":
                msgid += unescape(line[1:-1])
            elif field == "str":
                msgstr += unescape(line[1:-1])
    if msgid is not None:
        entries[msgid] = msgstr
    header = entries.pop("", "")
    return header, entries


def write(path, strings, translations, header):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write('msgid ""\nmsgstr ' + quote(header or "Content-Type: text/plain; charset=UTF-8\n") + "\n")
        for text, where in strings.items():
            f.write(f"\n#: {where}\nmsgid {quote(text)}\nmsgstr {quote(translations.get(text, ''))}\n")


def main():
    strings = collect()
    write(os.path.join(LANG, DOMAIN + ".pot"), strings, {}, "")
    for po in sorted(glob.glob(os.path.join(LANG, "*", "LC_MESSAGES", DOMAIN + ".po"))):
        header, old = read_po(po)
        write(po, strings, old, header)
        done = sum(1 for s in strings if old.get(s))
        print(f"{os.path.relpath(po, ROOT)}: {done}/{len(strings)} translated")
    print(f"{len(strings)} strings")


if __name__ == "__main__":
    main()
