#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail
if [[ "${TRACE-0}" == "1" ]]; then
    set -o xtrace
fi
if [[ "${1-}" =~ ^-*h(elp)?$ ]]; then
    echo 'Usage: ./gen_migrations.sh TODO Implement usage

This is an awesome bash script to make your life better.

'
    exit
fi
cd "$(dirname "$0")"

if test "$1" == ""; then
    echo "Please give the directory that will hold migrations as a first argument"
    exit 1
fi

mkdir "$1"
cd "$1"

if test "$2" == ""; then
    echo "Please give the number of migrations as a second argument"
    exit 2
fi
nbr_migrations="$2"

for i in $(seq 1 $nbr_migrations);
do
    dir="${i}-comment"
    mkdir "$dir"
    echo "CREATE TABLE t${i}(a, b, c);" >"$dir"/up.sql
    echo "DROP TABLE t${i};" >"$dir"/down.sql
done
