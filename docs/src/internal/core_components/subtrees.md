### Subtree Implementations

While [Entries](entry.md) store subtree data as raw serialized strings (`RawData`), specific implementations provide structured ways to interact with this data via the [`Operation`](basedb_tree.md) context. These implementations handle serialization/deserialization and provide typed APIs.

**Note on Naming:** Subtree names beginning with an underscore (e.g., `_settings`, `_root`) are reserved for internal Eidetica use. Avoid using this prefix for user-defined subtrees to prevent conflicts.

Currently, the main specialized implementations are `Table<T>`, `Dict`, and `YDoc` (requires the "y-crdt" feature).

<!-- TODO: Add a section on the `SubtreeType` trait and how new types can be created. -->

### The `SubTree` Trait

Specific subtree types (like `Table`, `Dict`, `YDoc`, or custom CRDT implementations) are accessed through handles that implement the [`SubTree`](../../src/subtree/mod.rs) trait. This trait requires:

- `fn new(op: &AtomicOp, subtree_name: impl AsRef<str>) -> Result<Self>`: An associated function used by `AtomicOp::get_subtree` to create a handle instance linked to the current operation.
- `fn name(&self) -> &str`: A method to retrieve the name of the subtree this handle manages.

Handles typically store a clone of the `AtomicOp` and the `subtree_name`.

To create a custom `SubTree` type:

1.  Define a struct for your handle.
2.  Implement the `SubTree` trait for it.
3.  Implement methods on your struct to provide the desired API (e.g., `get`, `set`, `add`).
4.  Internally, these methods will interact with the stored `AtomicOp`:
    - Use `op.get_local_data::<MyCRDT>()` to get the currently staged state for the operation.
    - Use `op.get_full_state::<MyCRDT>()` to get the merged historical state.
    - Use `op.update_subtree(self.name(), &serialized_new_state)` to stage updated CRDT data back into the operation.

#### Table<T>

`Table<T>` is a specialized subtree type designed for managing collections of records (structs `T`) where each record needs a unique, stable identifier.

```mermaid
classDiagram
    class Table~T~ {
        <<SubtreeType>>
        # Internal state likely uses HashMap<ID, T>
        +insert(value: T) Result<ID>
        +get(id: &str) Result<T>
        +set(id: &str, value: T) Result<()>
        +search(predicate: F) Result<Vec<(ID, T)>> where F: Fn(&T) -> bool
        # T must implement Serialize + Deserialize
    }
```

**Features:**

- **Record Management**: Stores instances of a user-defined type `T` (where `T: Serialize + Deserialize`).
- **Automatic ID Generation**: Automatically generates a unique UUID (as `String`) for each record inserted via `insert()`. This ID is used for subsequent `get()` and `set()` operations. Note: These are Table-specific record IDs, distinct from Eidetica's main `ID` type.
- **CRUD Operations**: Provides `insert`, `get`, `set`, and `search` methods for managing records.
- **Typed Access**: Accessed via `Operation::get_subtree::<Table<T>>("subtree_name")?`, providing type safety.

Internally, `Table<T>` manages its state (likely a map of IDs to `T` instances) and serializes it (e.g., to JSON) into the `RawData` field of the containing `Entry` when an `Operation` is committed.

<!-- TODO: Confirm the exact internal representation and serialization format. -->

`Table` is suitable for scenarios like managing a list of users, tasks (as in the Todo example), or any collection where individual items need to be addressed by a persistent ID.

#### Dict

`Dict` is a key-value store implementation that uses the `Map` CRDT to provide nested data structures and reliable deletion tracking across distributed systems.

```mermaid
classDiagram
    class Dict {
        <<SubtreeType>>
        +get<K>(key: K) Result<Value> where K: Into<String>
        +get_string<K>(key: K) Result<String> where K: Into<String>
        +set<K, V>(key: K, value: V) Result<()> where K: Into<String>, V: Into<String>
        +set_value<K>(key: K, value: Value) Result<()> where K: Into<String>
        +delete<K>(key: K) Result<()> where K: Into<String>
        +get_all() Result<Map>
        +get_value_mut<K>(key: K) ValueEditor where K: Into<String>
        +get_root_mut() ValueEditor
        +get_at_path<S, P>(path: P) Result<Value> where S: AsRef<str>, P: AsRef<[S]>
        +set_at_path<S, P>(path: P, value: Value) Result<()> where S: AsRef<str>, P: AsRef<[S]>
    }

    class ValueEditor {
        +new<K>(kv_store: &Dict, keys: K) Self where K: Into<Vec<String>>
        +get() Result<Value>
        +set(value: Value) Result<()>
        +get_value<K>(key: K) Result<Value> where K: Into<String>
        +get_value_mut<K>(key: K) ValueEditor where K: Into<String>
        +delete_self() Result<()>
        +delete_child<K>(key: K) Result<()> where K: Into<String>
    }

    Dict --> ValueEditor : creates
```

**Features:**

- **Flexible Data Structure**: Based on `Map`, which allows storing both simple string values and nested map structures.
- **Tombstone Support**: When a key is deleted, a tombstone is created to ensure the deletion propagates correctly during synchronization, even if the value doesn't exist in some replicas.
- **Key Operations**:

  - `get`: Returns the value for a key as a `Value` (String, Map, or error if deleted)
  - `get_string`: Convenience method that returns a string value (errors if the value is a map)
  - `set`: Sets a simple string value for a key
  - `set_value`: Sets any valid `Value` (String, Map, or Deleted) for a key
  - `delete`: Marks a key as deleted by creating a tombstone
  - `get_all`: Returns the entire store as a `Map` structure, including tombstones
  - `get_value_mut`: Returns a `ValueEditor` for modifying values at a specific key path
  - `get_root_mut`: Returns a `ValueEditor` for the root of the Dict's subtree
  - `get_at_path`: Retrieves a value at a specific nested path
  - `set_at_path`: Sets a value at a specific nested path

- **ValueEditor**: Provides a fluent API for navigating and modifying nested structures in the Dict:

  - Allows traversing into nested maps through method chaining
  - Supports reading, writing, and deleting values at any level of nesting
  - Changes made via ValueEditor are staged in the AtomicOp and must be committed to persist

- **Merge Strategy**: When merging two Dict states:
  - If both have string values for a key, the newer one wins
  - If both have map values, the maps are recursively merged
  - If types differ (map vs string) or one side has a tombstone, the newer value wins
  - Tombstones are preserved during merges to ensure proper deletion propagation

`Dict` is ideal for configuration data, metadata, and hierarchical data structures that benefit from nested organization. The tombstone mechanism ensures consistent behavior in distributed environments where deletions need to propagate reliably.

Example usage:

```rust
let op = tree.new_operation()?;
let kv = op.get_subtree::<Dict>("config")?;

// Set simple string values
kv.set("username", "alice")?;

// Create nested structures
let mut preferences = Map::new();
preferences.set_string("theme", "dark");
preferences.set_string("language", "en");
kv.set_value("user_prefs", Value::Map(preferences))?;

// Using ValueEditor to modify nested structures
let editor = kv.get_value_mut("user_prefs");
editor.get_value_mut("theme").set(Value::String("light".to_string()))?;
editor.get_value_mut("notifications").set(Value::String("enabled".to_string()))?;

// Using path-based APIs with string literals directly
kv.set_at_path(["user", "profile", "email"], Value::String("user@example.com".to_string()))?;
let email = kv.get_at_path(["user", "profile", "email"])?;

// Delete keys (creating tombstones)
kv.delete("old_setting")?;
// Or using ValueEditor
editor.delete_child("deprecated_setting")?;

// Commit changes
op.commit()?;
```

#### YDoc (Y-CRDT Integration)

`YDoc` provides seamless integration with Y-CRDT (Yjs) for real-time collaborative editing and automatic conflict resolution. This implementation is only available when the "y-crdt" feature is enabled.

```mermaid
classDiagram
    class YDoc {
        <<SubtreeType>>
        +doc() Result<Doc>
        +with_doc<F, R>(f: F) Result<R> where F: FnOnce(&Doc) -> Result<R>
        +with_doc_mut<F, R>(f: F) Result<R> where F: FnOnce(&Doc) -> Result<R>
        +apply_update(update_data: &[u8]) Result<()>
        +get_update() Result<Vec<u8>>
        +save_doc_full(doc: &Doc) Result<()>
        +save_doc(doc: &Doc) Result<()>
    }

    class YrsBinary {
        <<CRDT>>
        +new(data: Vec<u8>) Self
        +as_bytes() &[u8]
        +is_empty() bool
        +merge(&self, other: &Self) Result<Self>
    }

    YDoc --> YrsBinary : uses for storage
```

**Features:**

- **Real-time Collaboration**: Built on Y-CRDT algorithms for sophisticated conflict resolution and real-time collaborative editing
- **Differential Saving**: Only stores incremental changes, not full document state, optimizing storage overhead
- **Efficient Caching**: Caches expensive backend data retrieval operations to minimize I/O
- **Full Y-CRDT API**: Direct access to the complete `yrs` library functionality through the underlying `Doc`
- **Seamless Integration**: Works with Eidetica's atomic operation and viewer model

**Architecture:**

The `YDoc` integrates Y-CRDT with Eidetica's CRDT system through the `YrsBinary` wrapper, which implements the required `Data` and `CRDT` traits. Key architectural features include:

- **Caching Strategy**: Caches the expensive `get_full_state()` operation from the backend and constructs documents on-demand
- **Differential Updates**: When saving, calculates diffs relative to the current backend state rather than full snapshots
- **Binary Update Merging**: The `YrsBinary` CRDT applies both updates to a new Y-CRDT document and returns the merged state

**Key Operations:**

- `doc()`: Gets the current Y-CRDT document, merging all historical state
- `with_doc()` / `with_doc_mut()`: Provides safe access to the document within a closure
- `apply_update()`: Applies an external Y-CRDT update to the current document
- `get_update()`: Returns the current staged changes as a binary update
- `save_doc()`: Saves changes using differential updates (recommended)
- `save_doc_full()`: Saves the complete document state (for special cases)

**Merge Strategy:**

When merging two `YrsBinary` instances, both updates are applied to a new Y-CRDT document, and the resulting merged state is returned. This preserves Y-CRDT's sophisticated conflict resolution algorithms within Eidetica's merge operations.

**Performance Considerations:**

The implementation minimizes both I/O overhead and memory usage by:

- Caching backend data retrieval operations
- Constructing documents and state vectors on-demand from cached data
- Storing only incremental changes rather than full document snapshots

Example usage:

```rust
// Enable the y-crdt feature in Cargo.toml:
// eidetica = { version = "0.1", features = ["y-crdt"] }

use eidetica::subtree::YDoc;
use eidetica::y_crdt::{Map, Text, Transact};

let op = tree.new_operation()?;
let ydoc_store = op.get_subtree::<YDoc>("collaborative_doc")?;

// Work directly with the Y-CRDT document
ydoc_store.with_doc_mut(|doc| {
    let text = doc.get_or_insert_text("content");
    let metadata = doc.get_or_insert_map("metadata");

    let mut txn = doc.transact_mut();

    // Insert text collaboratively
    text.insert(&mut txn, 0, "Hello, collaborative world!");

    // Set metadata
    metadata.insert(&mut txn, "title", "My Document");
    metadata.insert(&mut txn, "author", "Alice");

    Ok(())
})?;

// Apply external updates from other clients
let external_update = receive_update_from_network();
ydoc_store.apply_update(&external_update)?;

// Get updates to send to other clients
let update_to_broadcast = ydoc_store.get_update()?;
send_update_to_network(update_to_broadcast);

// Commit changes (saves only the differential updates)
op.commit()?;
```

Other subtree types can be implemented, particularly those adhering to the [CRDT System](crdt.md).
