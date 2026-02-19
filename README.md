# Syster CLI

Command-line interface for SysML v2 and KerML analysis, interchange, and semantic model editing.

## Installation

```bash
cargo install syster-cli
```

Or build from source:

```bash
cargo build --release --features interchange
```

The `interchange` feature (enabled by default) adds model export/import, decompilation, and semantic editing commands.

## Usage

### Basic Analysis

```bash
# Analyze a single file
syster model.sysml

# Analyze a directory
syster ./models/

# With verbose output
syster -v model.sysml

# Standard library is loaded by default; skip with --no-stdlib
syster --no-stdlib model.sysml

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
syster model.sysml --export json-ld

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

# Decompile XMI back to SysML text + metadata
syster model.xmi --decompile
```

Decompilation produces two files:

- `model.sysml` — the reconstructed SysML text
- `model.metadata.json` — element ID mappings for round-trip fidelity

### Query and Inspect

Browse the semantic model without modifying it:

```bash
# List all elements in a model
syster model.sysml --list

# Search elements by name (substring match)
syster model.sysml --query Vehicle

# Filter by metaclass kind
syster model.sysml --list --kind PartDefinition
syster model.sysml --query mass --kind AttributeUsage

# Inspect a specific element (children, relationships, ID)
syster model.sysml --inspect Vehicle

# Inspect by qualified name
syster model.sysml --inspect "VehicleModel::Car::engine"

# JSON output for any command
syster model.sysml --list --json
syster model.sysml --inspect Vehicle --json
```

### Rename Elements

Rename a definition or usage across the model:

```bash
# Rename an element
syster model.sysml --rename Vehicle=Automobile -o updated.sysml
```

The renamed output is written to the specified file (or stdout). A companion `.metadata.json` file is written alongside the output so that element IDs are preserved through subsequent edits.

### Add Members

Add a new part, attribute, or other member to an existing element:

```bash
# Add an untyped part
syster model.sysml --add-member "Vehicle:PartUsage:chassis" -o updated.sysml

# Add a typed part
syster model.sysml --add-member "Vehicle:PartUsage:engine:Engine" -o updated.sysml

# Add an attribute
syster model.sysml --add-member "Vehicle:AttributeUsage:color" -o updated.sysml
```

The format is `PARENT:KIND:NAME[:TYPE]`. Supported kinds:

| Kind string | Element kind |
|-------------|-------------|
| `PartUsage`, `part` | Part usage |
| `PartDefinition`, `part_def` | Part definition |
| `AttributeUsage`, `attribute`, `attr` | Attribute usage |
| `AttributeDefinition`, `attr_def` | Attribute definition |
| `PortUsage`, `port` | Port usage |
| `PortDefinition`, `port_def` | Port definition |
| `ItemUsage`, `item` | Item usage |
| `ItemDefinition`, `item_def` | Item definition |
| `Package` | Package |

### Remove Elements

Remove an element from the model by name or qualified name:

```bash
# Remove by simple name
syster model.sysml --remove-member wheels -o updated.sysml

# Remove by qualified name
syster model.sysml --remove-member "VehicleModel::Car::wheels" -o updated.sysml
```

## Metadata and ID Preservation

When working with interchange formats, element identity (UUIDs) matters. The CLI uses companion `.metadata.json` files to preserve element IDs across edits:

1. **Export** a model to XMI — each element gets a stable UUID.
2. **Decompile** the XMI back to SysML — a `.metadata.json` file records the original IDs.
3. **Edit** the SysML (rename, add, remove) — the CLI reads the companion metadata and carries IDs forward.
4. **Re-export** — previously-known elements retain their original UUIDs.

```bash
# Full round-trip example
syster model.sysml --export xmi -o model.xmi
syster model.xmi --decompile -o model.sysml          # creates model.metadata.json
syster model.sysml --rename Car=Sedan -o model.sysml  # updates model.metadata.json
syster model.sysml --add-member "Sedan:PartUsage:sunroof" -o model.sysml
syster model.sysml --export xmi -o model.xmi          # IDs preserved
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

## All Options

```
syster [OPTIONS] <FILE>

Arguments:
  <FILE>              Input file or directory to analyze

Options:
  -v, --verbose       Enable verbose output
  -o, --output <FILE> Write output to file instead of stdout
      --json          Output results as JSON
      --no-stdlib     Skip loading standard library
      --stdlib-path   Path to custom standard library

Analysis:
      --export-ast    Export AST as JSON

Interchange (requires interchange feature):
      --export <FMT>    Export model (xmi, yaml, json-ld, kpar)
      --self-contained  Include stdlib in export
      --import          Import and validate interchange file
      --import-workspace Import into workspace for analysis
      --decompile       Decompile interchange to SysML + metadata

Semantic Model:
      --list            List all model elements
      --query <NAME>    Search elements by name (substring)
      --kind <KIND>     Filter by metaclass kind
      --inspect <NAME>  Inspect element details
      --rename <OLD=NEW>  Rename an element
      --add-member <PARENT:KIND:NAME[:TYPE]>  Add a member element
      --remove-member <NAME>  Remove an element
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success — no errors |
| `1` | Error — parse failure, missing element, invalid input, or hard validation error |
| `2` | Success with warnings — e.g. `--import` with unresolved stdlib type references |

## Features

- Parse and validate SysML v2 and KerML files
- Symbol table analysis with qualified names
- Import resolution and type checking
- Error reporting with source locations
- Export to XMI, YAML, JSON-LD, and KPAR formats
- Import and validate interchange files
- Decompile XMI back to SysML text with metadata
- Self-contained export with embedded stdlib
- Semantic model queries (list, search, inspect)
- In-place model editing (rename, add member, remove)
- Element ID preservation across edit round-trips via companion metadata

## Supported Formats

| Format | Extension | Description |
|--------|-----------|-------------|
| XMI | `.xmi` | OMG XML Metadata Interchange (standard) |
| YAML | `.yaml` | Human-readable YAML representation |
| JSON-LD | `.jsonld` | JSON Linked Data format |
| KPAR | `.kpar` | Kernel Package Archive (ZIP) |

## License

MIT
