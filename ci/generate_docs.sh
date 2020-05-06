#!/bin/sh
VERSION="v$(cat ./Cargo.toml | grep "version" | head -n 1 | awk '{print $3}' | cut -d "\"" -f 2)"
mkdir docs
sed -e "s/{{ version }}/$VERSION/g" ./ci/index.html.template > ./docs/index.html
