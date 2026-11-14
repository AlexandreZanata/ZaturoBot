#!/usr/bin/env python3
"""Install only the local MCP adapter/profile, reusing the existing scoped desktop credential."""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import tomllib


def load_adapter():
    spec=importlib.util.spec_from_file_location("zaturo_adapter",Path(__file__).with_name("zaturo_mcp.py"))
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module


def safe(path):
    if any(p.is_symlink() for p in [path,*path.parents]):raise ValueError("symlink installation path rejected")


def atomic(path,data,mode):
    safe(path);path.parent.mkdir(mode=0o700,parents=True,exist_ok=True)
    if path.exists() and not path.is_file():raise ValueError("regular installation file required")
    fd,temporary=tempfile.mkstemp(prefix=".zaturo-mcp-",dir=path.parent)
    try:
        with os.fdopen(fd,"wb") as stream:
            os.fchmod(stream.fileno(),mode);stream.write(data);stream.flush();os.fsync(stream.fileno())
        os.replace(temporary,path)
    finally:
        if os.path.exists(temporary):os.unlink(temporary)


def install(home,apply=False):
    home=home.resolve(strict=True);adapter=load_adapter()
    profile=home/".config/zaturobot/desktop.toml"
    value=tomllib.loads(adapter.private_bytes(profile).decode())
    configuration=("hub_url="+json.dumps(value["hub_url"])+"\nenv_file="+json.dumps(value["credentials_file"])+"\n").encode()
    destination=home/".local/lib/zaturobot/zaturo_mcp.py";config=home/".config/zaturobot/mcp.toml"
    # Check the proposed profile using a private temporary file before touching the installation.
    fd,path=tempfile.mkstemp(prefix="zaturo-mcp-plan-")
    try:
        with os.fdopen(fd,"wb") as stream:os.fchmod(stream.fileno(),0o600);stream.write(configuration)
        adapter.Hub(Path(path))
    finally:os.unlink(path)
    source=Path(__file__).with_name("zaturo_mcp.py").read_bytes()
    if apply:
        for target in [destination,config]:
            safe(target)
            if target.exists() and target.read_bytes()!=(source if target==destination else configuration):
                digest=hashlib.sha256(target.read_bytes()).hexdigest()[:12]
                backup=target.with_name(target.name+".before-"+digest)
                if not backup.exists():atomic(backup,target.read_bytes(),0o600)
        atomic(destination,source,0o700);atomic(config,configuration,0o600)
    return {"applied":apply,"adapter":str(destination),"config":str(config),"credential_reused":True,"codex_command":["codex","mcp","add","zaturobot","--","/usr/bin/python3",str(destination),"--config",str(config)]}


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--home",type=Path,default=Path.home());parser.add_argument("--apply",action="store_true")
    args=parser.parse_args();os.umask(0o077)
    try:print(json.dumps(install(args.home,args.apply),indent=2));return 0
    except (OSError,ValueError,KeyError,tomllib.TOMLDecodeError):print("MCP installation rejected; inspect the existing private desktop profile without displaying credentials.");return 1

if __name__=="__main__":raise SystemExit(main())
