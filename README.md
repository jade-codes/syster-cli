# Syster CLI

Command-line interface for SysML v2 and KerML analysis and interchange.

## Installation

```bash
cargo install syster-cli
```

Or build from source:

```bash
cargo build --release
```

## Usage

### Basic Analysis

```bash
# Analyze a single file
syster model.sysml

# Analyze a directory
syster ./models/

# With verbose output
syster -v model.sysml

# With standard library
syster --stdlib model.sysml

# Custom stdlib path
syster --stdlib-path /path/to/sysml.library model.sysml
```

### Export Formats

Export models to various interchange formats:

```bash
# Export to XMI (OMG standard)
syster model.sysml --export xmi

# Export to YAML (human-readable)
syster model.sysml --export yaml

# Export to JSON-LD (linked data)
syster model.sysml --export jsonld

# Export to KPAR (Kernel Package Archive)
syster model.sysml --export kpar -o model.kpar

# Export AST as JSON
syster model.sysml --export-ast

# Self-contained export (includes stdlib)
syster model.sysml --export xmi --self-contained
```

### Import and Roundtrip

```bash
# Import and validate an XMI file
syster model.xmi --import

# Import into workspace for analysis
syster model.xmi --import-workspace

# Decompile XMI back to SysML text
syster model.xmi --decompile
```

## Export Format Examples

Given this SysML input:

```sysml
part def Vehicle {
    attribute mass : Real;
}
```

### XMI Output

```xml
<?xml version="1.0" encoding="ASCII"?>
<sysml:PartDefinition 
    xmlns:sysml="https://www.omg.org/spec/SysML/20250201"
    xmi:id="68e00c54-9196-421b-9149-76783d5c26f5"
    declaredName="Vehicle" 
    qualifiedName="Vehicle">
  <ownedRelatedElement xsi:type="sysml:AttributeUsage"
      xmi:id="15e06b2e-7efc-4d4a-8c66-670ce186f57f"
      declaredName="mass" 
      qualifiedName="Vehicle::mass"/>
</sysml:PartDefinition>
```

### YAML Output

```yaml
- '@type': PartDefinition
  '@id': cc10f11d-996f-4251-8952-9723018b762d
  name: Vehicle
  qualifiedName: Vehicle
  ownedMember:
    - '@id': 48e432b9-fdfe-483a-bd2d-36e6417703b2

- '@type': AttributeUsage
  '@id': 48e432b9-fdfe-483a-bd2d-36e6417703b2
  name: mass
  qualifiedName: Vehicle::mass
  owner:
    '@id': cc10f11d-996f-4251-8952-9723018b762d

- '@type': FeatureTyping
  '@id': rel_1
  source:
    '@id': 48e432b9-fdfe-483a-bd2d-36e6417703b2
  target:
    '@id': Real
```

### AST JSON Output (`--export-ast`)

```json
{
  "files": [
    {
      "path": "model.sysml",
      "symbols": [
        {
          "name": "Vehicle",
          "qualified_name": "Vehicle",
          "kind": "PartDefinition",
          "start_line": 1,
          "start_col": 10,
          "supertypes": ["Parts::Part"]
        },
        {
          "name": "mass",
          "qualified_name": "Vehicle::mass",
          "kind": "AttributeUsage",
          "supertypes": ["Real"]
        }
      ]
    }
  ]
}
```

## Features

- Parse and validate SysML v2 and KerML files
- Symbol table analysis with qualified names
- Import resolution and type checking
- Error reporting with source locations
- Export to XMI, YAML, JSON-LD, and KPAR formats
- Import and validate interchange files
- Decompile XMI back to SysML text
- Self-contained export with embedded stdlib

## Supported Formats

| Format | Extension | Description |
|--------|-----------|-------------|
| XMI | `.xmi` | OMG XML Metadata Interchange (standard) |
| YAML | `.yaml` | Human-readable YAML representation |
| JSON-LD | `.jsonld` | JSON Linked Data format |
| KPAR | `.kpar` | Kernel Package Archive (ZIP) |

## License

MIT
