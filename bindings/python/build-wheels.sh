#!/usr/bin/env sh
docker pull konstin2/maturin
docker run --rm -v $(pwd):/io konstin2/maturin build --release
