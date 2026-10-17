"""Neutrality of generated data (ARCHITEKTUR §1, docs\\AGENTEN_REGELN.md §1a).

The names of the original and its vendor appear nowhere in the Firepixel source. Generators that take
texts from the installation copy or from ..\\re\\ replace them with `neutral()` or drop records with
`contains()`. The names are read from the installation copy at run time:

  program name = file name (without extension) of the main EXE (the EXE with the resource CMYK/1001,
                 recognised by the UTF-16 resource name; the largest EXE in the folder),
  vendor       = `CompanyName` from the version resource of that EXE.

Corpus folder: environment variable FP_CORPUS_DIR, otherwise <REA>\\korpus (directory junction to the
installation copy).
"""
import os
import re

REA = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
CORPUS = os.environ.get("FP_CORPUS_DIR") or os.path.join(REA, "korpus")

_names = None


def _main_exe(root):
    exes = [os.path.join(root, f) for f in os.listdir(root) if f.lower().endswith(".exe")]
    exes.sort(key=os.path.getsize, reverse=True)
    for p in exes:
        with open(p, "rb") as fh:
            data = fh.read()
        if "CMYK".encode("utf-16-le") in data:
            return p, data
    raise FileNotFoundError("no main EXE in corpus " + root)


def _version_string(data, key):
    k = (key + "\0").encode("utf-16-le")
    i = data.find(k)
    if i < 0:
        return None
    i += len(k)
    while i % 4:  # align to 32 bits
        i += 1
    end = data.find(b"\0\0", i)
    while end >= 0 and (end - i) % 2:
        end = data.find(b"\0\0", end + 1)
    return data[i:end].decode("utf-16-le", errors="replace").strip() or None


def names():
    """(program name, vendor) from the installation copy."""
    global _names
    if _names is None:
        exe, data = _main_exe(CORPUS)
        app = os.path.splitext(os.path.basename(exe))[0]
        vendor = _version_string(data, "CompanyName") or ""
        vendor = vendor.split()[0] if vendor else ""
        _names = (app, vendor)
    return _names


def contains(text):
    """Does the text contain one of the two names (in any case)?"""
    app, vendor = names()
    pats = [re.escape(n) for n in (app, vendor) if n]
    return bool(pats) and re.search("(?i)" + "|".join(pats), text) is not None


def neutral(text):
    """Replaces the names: vendor + program or program -> "Firepixel" (case preserved); the vendor
    as a prefix ("<Vendor>-Onlineforen", "<VENDOR> AI3") is dropped."""
    app, vendor = names()
    if vendor:
        text = re.sub(re.escape(vendor) + r"[ -](?=" + re.escape(app) + ")", "", text, flags=re.I)
    def rep(m):
        s = m.group(0)
        if s.isupper():
            return "FIREPIXEL"
        if s[0].islower():
            return "firepixel"
        return "Firepixel"
    text = re.sub(re.escape(app), rep, text, flags=re.I)
    if vendor:
        text = re.sub(re.escape(vendor) + r"[ -]?", "", text, flags=re.I)
    return text


if __name__ == "__main__":
    a, v = names()
    print("program name found:", bool(a), "vendor found:", bool(v))
    print(neutral("Test"))
