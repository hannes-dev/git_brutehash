
./make_test_dir.sh
cd test_dir

cargo build --profile profile
perf record --call-graph dwarf ../target/profile/brutecommit -d 00000 -t1
cp perf.data $1.data
