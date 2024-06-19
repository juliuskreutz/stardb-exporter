import os
import re
import json
import shutil
import urllib.request

# Using system commands cause lazy
os.system("git clone https://gitlab.com/Melledy/LunarCore-Protos.git")
os.system("git clone https://github.com/IceDynamix/reliquary.git")
os.system("git clone https://github.com/IceDynamix/reliquary-codegen.git")

os.mkdir("data")

urllib.request.urlretrieve(
    "https://raw.githubusercontent.com/Melledy/LunarCore/development/src/main/java/emu/lunarcore/server/packet/CmdId.java",
    "cmdid.java",
)
# stolen with love from https://github.com/hashblen
dict = {}
with open("cmdid.java", "r") as f:
    while f.readline().strip() != "// Cmd Ids":
        pass
    for line in f.readlines():
        line = line.strip()
        name_match = re.search(r"(?<=public static final int )\w+(?= )", line)
        id_match = re.search(r"(?<== )[0-9]+(?=;)", line)
        if name_match is None or id_match is None:
            continue
        dict[id_match[0]] = name_match[0]
with open("data/packetIds.json", "w") as f:
    json.dump(dict, f)
# stolen with love from https://github.com/hashblen

shutil.copytree("LunarCore-Protos/proto", "data/proto")

os.system("cd reliquary-codegen && cargo run -- ../reliquary ../data")
