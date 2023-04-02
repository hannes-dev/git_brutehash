#!/usr/bin/env bash

if [ -d "./test_dir" ]; then
    echo "test_dir directory already exists, remaking"
    rm -rf ./test_dir
fi


mkdir ./test_dir
cd test_dir
git init
echo test >> test.txt
git add test.txt
GIT_COMMITTER_DATE="2024-01-01 00:00:00" GIT_COMMITTER_NAME="Profile" GIT_COMMITTER_EMAIL="profile@example.com" git commit --author="Profile <profile@example.com>" --no-edit --date="2024-01-01 00:00:00" -m "profile commit"
git log --pretty=fuller

if ! [ $(git rev-parse HEAD) = "6acea39b49eb4558715d28836fcd434d85d06ae2" ]; then
    echo "Commit hash of profile commit is not as expected"
    exit 1
fi