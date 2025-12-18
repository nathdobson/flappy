#!/usr/bin/env bash
set -e
set -u

VERSION=$1

git tag -a $1 -m $1
git push origin $1
gh release create $1 --title $1 --notes ""
