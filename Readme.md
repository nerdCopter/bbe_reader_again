## Experiment in A.I to create a RUST program to read Betaflight Blackbox PID, FF

### Prerequisite

https://www.rust-lang.org/tools/install

### Build and execute

```shell
clear
cargo build --release && ./target/release/bbe_reader_again --input path/to/BTFL_Log.BBL
ls -lhrt *.png *.csv
```

As it sits, it produces nonsense data. WIP.
