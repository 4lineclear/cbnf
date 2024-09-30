test:
    cargo nextest run

md:
    mdflc ./notes/

clippy:
    cargo clippy --workspace

check:
    cargo check --workspace

todo:
    rg "todo|FIX|TODO|HACK|WARN|PERF|NOTE|TEST" ./cbnf/ ./cbnf-ls/

cov:
    cargo llvm-cov --html
