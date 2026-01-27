# Syster CLI

Command-line interface for SysML v2 and KerML analysis.

Build from source:

```bash
cargo build --release
```

## Usage

```bash
# Analyze a single file
syster model.sysml

# Analyze a directory
syster ./models/

# With verbose output
syster -v model.sysml

# Custom stdlib path
syster --stdlib-path /path/to/sysml.library model.sysml
```

## Features

- Parse and validate SysML v2 and KerML files
- Symbol table analysis
- Import resolution
- Error reporting with source locations

## License

MIT
