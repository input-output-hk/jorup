#!/bin/sh
VERSION=$1
mkdir docs
sed -e "s/{{ version }}/$VERSION/g" ./ci/index.html.template > ./docs/index.html
