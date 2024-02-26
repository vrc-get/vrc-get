This crate is to ensure built binary is statically linked or dynamically linked to the system libraries.

this crate use `object` crate to read the binary file.

## Usage

```
cargo run -p build-check-static-link <path/to/binary>
```

exits with zero if statically linked or linked with allowed dynamic libraries, otherwise exits with non-zero.
