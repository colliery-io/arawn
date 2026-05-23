"""Build commands for Arawn."""

import os
import subprocess

import angreal
from angreal.integrations.flox import Flox

build = angreal.command_group(name="build", about="Build the project")


@build()
@angreal.command(name="workspace", about="Build the workspace")
@angreal.argument(name="release", long="release", is_flag=True, takes_value=False, help="Build in release mode")
def build_workspace(release=False):
    """Build all workspace crates."""
    with Flox("."):
        cmd = ["cargo", "build", "--workspace"]
        if release:
            cmd.append("--release")
        subprocess.run(cmd, check=True)


