"""Shared helpers of the fp-text golden generators (read-only access to re/ and the corpus)."""
import base64
import os
import sys

RE = r"C:\Users\Administrator\Desktop\REA\re"
CORPUS = os.environ.get("FP_CORPUS_DIR") or r"C:\Users\Administrator\Desktop\REA\korpus"
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "out")

for p in (os.path.join(RE, "text"), os.path.join(RE, "format"), os.path.join(RE, "scripts", "text")):
    if p not in sys.path:
        sys.path.insert(0, p)


def leaves(n):
    return {c.key: c.value for c in n.children() if c.type != "v"}


def runs_of(n):
    """TFSv runs like scripts/text/extract_text.py: one dict per attribute block + TRNs."""
    runs = []
    for tfs in n.children("TFSv"):
        cur = {}
        for c in tfs.children():
            if c.type == "v":
                continue
            cur[c.key] = c.value
            if c.key == "TRNs":
                runs.append(cur)
                cur = {}
        if cur:
            runs.append(cur)
    return runs


def path_contours(pthv):
    """PTHv -> list of contours {closed, nodes: [[x, y, xp, yp, xs, ys], ...]} (anchor, in handle, out handle)."""
    out = []
    pbl = pthv.child("PBLv") if pthv is not None else None
    if pbl is None:
        return out
    for pbp in pbl.children("PBPv"):
        nodes = []
        for pbt in pbp.children("PBTv"):
            x, y = pbt.get("XLCf", 0.0), pbt.get("YLCf", 0.0)
            nodes.append([x, y, pbt.get("XPCf", x), pbt.get("YPCf", y), pbt.get("XSCf", x), pbt.get("YSCf", y)])
        out.append({"closed": bool(pbp.get("ISCb", False)), "nodes": nodes})
    return out


def b64(data):
    return base64.b64encode(bytes(data)).decode("ascii")
