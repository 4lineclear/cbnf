test *FLAGS:
    cargo nextest run {{FLAGS}}

md:
    mdflc ./notes/

clippy *FLAGS:
    cargo clippy --workspace {{FLAGS}} 

build *FLAGS:
    cargo check --workspace {{FLAGS}} 

check *FLAGS:
    cargo check --workspace {{FLAGS}} 

todo:
    rg "todo|FIX|FIXME|TODO|HACK|WARN|PERF|NOTE|TEST" ./cbnf/ ./cbnf-ls/

cov:
    cargo llvm-cov --html
