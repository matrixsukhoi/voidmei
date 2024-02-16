#!/usr/bin/env python3

import subprocess
import shlex
import shutil
import sys
import xml.etree.ElementTree as ET
import os

RESOURCES = ["config", "data", "fonts", "image", "language", "records", "voice"]


def yellow(*args, **kwargs):
    return "\033[0;33m" + " ".join(args) + "\033[0m"


def green(*args, **kwargs):
    return "\033[0;32m" + " ".join(args) + "\033[0m"


def red(*args, **kwargs):
    return "\033[0;31m" + " ".join(args) + "\033[0m"


def execute(cmd: str, timeout: int = 60, capture_output: bool = False):
    print(yellow("----------------------"))
    print(yellow("execute: " + " ".join(shlex.split(cmd))))
    try:
        # by default `subprocess.run` wont look into PATH
        pargs = shlex.split(cmd)
        program = shutil.which(pargs[0])
        if not program:
            print(red("no " + pargs[0] + " found, abort. "))
            sys.exit(-1)
        if program != None:
            pargs[0] = program

        result = subprocess.run(
            pargs,
            timeout=timeout,
            capture_output=capture_output,
            text=True,
        )
        if capture_output:
            return result.stdout + result.stderr
        else:
            return ""

    except subprocess.TimeoutExpired:
        print(red("timeout"))
        sys.exit(-1)


def build(args):
    execute("echo conosuba")

    # dependency
    result = execute(
        'mvn dependency:get -Dartifact="com.voidmei:weblaf-complete:1.29"',
        capture_output=True,
    )
    if "BUILD FAILURE" in result:
        execute(
            'mvn install:install-file -Dfile="src/main/resources/weblaf-complete-1.29.jar" -DgroupId="com.voidmei" -DartifactId=weblaf-complete -Dversion="1.29" -Dpackaging=jar'
        )
    elif "BUILD SUCCESS" in result:
        print(green("weblaf-complete exists in local maven repository. "))

    # resources
    if not all(map(os.path.isdir, RESOURCES)):
        clean()
        execute("7z x ./resources.7z")

    # compile
    execute("mvn package")

    # run
    root = ET.parse("pom.xml").getroot()
    version = root.find("{http://maven.apache.org/POM/4.0.0}version").text
    execute("java -jar ./target/voidmei-" + version + ".jar", timeout=None)


# 7z a -t7z resources.7z -mx9 config/* data/* fonts/* image/* language/* records/* voice/*


def clean(args=None):
    execute("git clean -dfx")


def publish(args):
    execute(
        "7z a -t7z resources.7z -mx9 config data fonts image language records voice", timeout=None
    )
        "7z a -t7z voidmei.7z -stl -mx9 config data fonts image language records voice README.md ReadMe.txt voidmei.exe voidmei.jar 更新日志.txt 计算说明.md 使用说明.txt", timeout=None)
        

def run(args):
    args.func(args)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="a build script.")
    subparsers = parser.add_subparsers(required=True, help="sub-command help")

    parserbuild = subparsers.add_parser("build", help="build")
    parserbuild.set_defaults(func=build)

    parserclean = subparsers.add_parser("clean", help="clean")
    parserclean.set_defaults(func=clean)

    parserpublish = subparsers.add_parser("publish", help="publish")
    parserpublish.set_defaults(func=publish)

    args = parser.parse_args()

    try:
        run(args)
    except KeyboardInterrupt:
        print(yellow("INTERRUPTED"))
