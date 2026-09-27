import json, re, sys, os

log_path = sys.argv[1]

t = open(log_path, errors="replace").read()
t = re.sub(r"\x1b\[[0-9;]*m", "", t).replace("\r", "")
fence = chr(96) * 3
body = fence + "\n" + t[-55000:] + "\n" + fence
json.dump({"body": body}, open("comment_body.json", "w"))
print("wrote comment_body.json, log length:", len(t))
