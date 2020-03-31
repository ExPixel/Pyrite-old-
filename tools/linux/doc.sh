#!/bin/bash

# set -x

documented_packages_list=(\
    imgui \
    glutin \
    pyrite-arm \
    pyrite-common \
    pyrite-gba \
    pyrite)

documented_packages=""

for p in "${documented_packages_list[@]}"; do
    documented_packages="${documented_packages} -p ${p}"
done

cargo doc --no-deps $documented_packages "${@:1}"
