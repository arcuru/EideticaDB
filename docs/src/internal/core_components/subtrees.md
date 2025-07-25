# Subtrees

Typed data access patterns within trees providing structured interaction with Entry RawData.

## Core Concepts

**SubTree Trait**: Interface for typed subtree implementations accessed through Operation handles.

**Reserved Names**: Subtree names with underscore prefix (e.g., `_settings`) reserved for internal use.

**Typed APIs**: Handle serialization/deserialization and provide structured access to raw entry data.

## Current Implementations

### Table<T>

Record-oriented store for managing collections with unique identifiers.

**Features**:

- Stores user-defined types (T: Serialize + Deserialize)
- Automatic UUID generation for records
- CRUD operations: insert, get, set, search
- Type-safe access via Operation::get_subtree

**Use Cases**: User lists, task management, any collection requiring persistent IDs.

### Dict

Key-value store using Map CRDT for nested structures and reliable deletion tracking.

**Features**:

- Nested data structures via Value enum (String, Map, Deleted)
- Tombstone support for distributed deletion propagation
- ValueEditor for fluent nested navigation
- Path-based APIs for deep value access
- Last-write-wins merge strategy with recursive merging

**Operations**:

- Basic: get, set, delete
- Advanced: nested path access, tombstone management
- Editor: fluent API for complex modifications

**Use Cases**: Configuration data, metadata, hierarchical structures.

### YDoc (Y-CRDT Integration)

Real-time collaborative editing with sophisticated conflict resolution.

**Features** (requires "y-crdt" feature):

- Y-CRDT algorithms for collaboration
- Differential saving for storage efficiency
- Full Y-CRDT API access
- Caching for performance optimization

**Architecture**:

- YrsBinary wrapper implements CRDT traits
- Differential updates vs full snapshots
- Binary update merging preserves Y-CRDT algorithms

**Operations**:

- Document access with safe closures
- External update application
- Incremental change tracking

**Use Cases**: Collaborative documents, real-time editing, complex conflict resolution.

## Custom SubTree Implementation

Requirements:

1. Struct implementing SubTree trait
2. Handle creation linked to AtomicOp
3. Custom API methods using AtomicOp interaction:
   - get_local_data for staged state
   - get_full_state for merged historical state
   - update_subtree for staging changes

## Integration

**Operation Context**: All subtrees accessed through atomic operations

**CRDT Support**: Subtrees can implement CRDT trait for conflict resolution

**Serialization**: Data stored as RawData strings in Entry structure
