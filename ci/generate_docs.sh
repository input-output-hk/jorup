#!/bin/sh
TAG_COMMIT=$(git rev-list --abbrev-commit --tags --max-count=1)
VERSION=$(git describe --abbrev=0 --tags $TAG_COMMIT 2>/dev/null || true)
mkdir docs
sed -e "s/{{ version }}/$VERSION/g" ./ci/index.html.template > ./docs/index.html
