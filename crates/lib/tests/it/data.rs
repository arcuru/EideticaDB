use crate::helpers::*;
use eidetica::Error;
use eidetica::atomicop::AtomicOp;
use eidetica::backend::database::InMemory;
use eidetica::basedb::BaseDB;
use eidetica::crdt::CRDT;
use eidetica::crdt::Map;
use eidetica::crdt::{Nested, Value};
use eidetica::entry::Entry;
use eidetica::subtree::KVStore;
use std::collections::HashMap;

#[test]
fn test_kvoverwrite_basic_operations() {
    let mut kv = create_kvoverwrite(&[]);

    // Test set and get
    let key = "test_key";
    let value = "test_value";
    kv.set(key, value);

    assert_eq!(kv.get(key), Some(&value.to_string()));
    assert_eq!(kv.get("non_existent_key"), None);

    // Test overwrite
    let new_value = "new_value";
    kv.set(key, new_value);
    assert_eq!(kv.get(key), Some(&new_value.to_string()));
}

#[test]
fn test_kvoverwrite_merge() {
    let kv1 = create_kvoverwrite(&[("key1", "value1"), ("key2", "value2")]);

    let kv2 = create_kvoverwrite(&[("key2", "value2_updated"), ("key3", "value3")]);

    // Merge kv2 into kv1
    let merged = kv1.merge(&kv2).expect("Merge failed");

    // Verify merged result
    assert_eq!(merged.get("key1"), Some(&"value1".to_string()));
    assert_eq!(merged.get("key2"), Some(&"value2_updated".to_string())); // overwritten
    assert_eq!(merged.get("key3"), Some(&"value3".to_string())); // added from kv2
}

#[test]
fn test_kvoverwrite_serialization() {
    let kv = create_kvoverwrite(&[("key1", "value1"), ("key2", "value2")]);

    // Serialize to string
    let serialized = serde_json::to_string(&kv).expect("Serialization failed");
    assert!(!serialized.is_empty());

    // Deserialize back
    let deserialized: Map = serde_json::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(deserialized.get("key1"), Some(&"value1".to_string()));
    assert_eq!(deserialized.get("key2"), Some(&"value2".to_string()));
}

#[test]
fn test_kvoverwrite_from_entry() {
    // Create an entry with Map data
    let kv = create_kvoverwrite(&[("key1", "value1"), ("key2", "value2")]);

    let serialized = serde_json::to_string(&kv).expect("Serialization failed");

    let entry_with_settings = Entry::root_builder()
        .set_subtree_data("tmp", serialized)
        .build();

    // Now we can get the data from the tmp subtree directly
    let data = entry_with_settings
        .data("tmp")
        .expect("Failed to get tmp data");
    let deserialized: Map = serde_json::from_str(data).expect("Deserialization failed");

    assert_eq!(deserialized.get("key1"), Some(&"value1".to_string()));
    assert_eq!(deserialized.get("key2"), Some(&"value2".to_string()));
}

#[test]
fn test_kvoverwrite_to_raw_data() {
    let kv = create_kvoverwrite(&[("key1", "value1")]);

    let raw_data = serde_json::to_string(&kv).expect("Serialization failed");
    assert!(!raw_data.is_empty());

    // Should be valid JSON
    let json_result = serde_json::from_str::<serde_json::Value>(&raw_data);
    assert!(json_result.is_ok());
}

#[test]
fn test_kvoverwrite_multiple_merge_operations() {
    // Start with an initial Map
    let base = create_kvoverwrite(&[
        ("key1", "initial1"),
        ("key2", "initial2"),
        ("common", "base"),
    ]);

    // Create two diverging updates
    let branch1 = create_kvoverwrite(&[
        ("key1", "branch1_value"),
        ("branch1_key", "branch1_only"),
        ("common", "branch1"),
    ]);

    let branch2 = create_kvoverwrite(&[
        ("key2", "branch2_value"),
        ("branch2_key", "branch2_only"),
        ("common", "branch2"),
    ]);

    // Merge in different orders to compare last-write-wins behavior

    // Order: base -> branch1 -> branch2
    let merged1 = base.merge(&branch1).expect("First merge failed");
    let merged1_2 = merged1.merge(&branch2).expect("Second merge failed");

    // Order: base -> branch2 -> branch1
    let merged2 = base.merge(&branch2).expect("First merge failed");
    let merged2_1 = merged2.merge(&branch1).expect("Second merge failed");

    // Since branch1 and branch2 modify different keys (except for "common"),
    // merged1_2 and merged2_1 should be mostly identical

    assert_eq!(merged1_2.get("key1"), Some(&"branch1_value".to_string()));
    assert_eq!(merged1_2.get("key2"), Some(&"branch2_value".to_string()));
    assert_eq!(
        merged1_2.get("branch1_key"),
        Some(&"branch1_only".to_string())
    );
    assert_eq!(
        merged1_2.get("branch2_key"),
        Some(&"branch2_only".to_string())
    );

    assert_eq!(merged2_1.get("key1"), Some(&"branch1_value".to_string()));
    assert_eq!(merged2_1.get("key2"), Some(&"branch2_value".to_string()));
    assert_eq!(
        merged2_1.get("branch1_key"),
        Some(&"branch1_only".to_string())
    );
    assert_eq!(
        merged2_1.get("branch2_key"),
        Some(&"branch2_only".to_string())
    );

    // But for the "common" key, the order matters
    assert_eq!(merged1_2.get("common"), Some(&"branch2".to_string())); // Last write wins
    assert_eq!(merged2_1.get("common"), Some(&"branch1".to_string())); // Last write wins
}

#[test]
fn test_kvoverwrite_serialization_roundtrip_with_merge() {
    // Create and serialize original data
    let original = create_kvoverwrite(&[("key1", "value1"), ("key2", "value2")]);

    let serialized = serde_json::to_string(&original).expect("Serialization failed");

    // Deserialize to a new instance
    let deserialized: Map = serde_json::from_str(&serialized).expect("Deserialization failed");

    // Create a second Map with different data
    let update = create_kvoverwrite(&[("key2", "updated2"), ("key3", "value3")]);

    // Merge update into the deserialized data
    let merged = deserialized.merge(&update).expect("Merge failed");

    // Serialize the merged result
    let merged_serialized =
        serde_json::to_string(&merged).expect("Serialization of merged data failed");

    // Deserialize again
    let final_data: Map =
        serde_json::from_str(&merged_serialized).expect("Deserialization of merged data failed");

    // Verify final state
    assert_eq!(final_data.get("key1"), Some(&"value1".to_string())); // Unchanged
    assert_eq!(final_data.get("key2"), Some(&"updated2".to_string())); // Updated
    assert_eq!(final_data.get("key3"), Some(&"value3".to_string())); // Added

    // Test merging with an empty CRDT
    let empty = Map::new();
    let merged_with_empty = final_data.merge(&empty).expect("Merge with empty failed");

    // Merging with empty should not change anything
    assert_eq!(merged_with_empty.get("key1"), Some(&"value1".to_string()));
    assert_eq!(merged_with_empty.get("key2"), Some(&"updated2".to_string()));
    assert_eq!(merged_with_empty.get("key3"), Some(&"value3".to_string()));
}

#[test]
fn test_kvoverwrite_new() {
    // Test creation of a new Map
    let kv = Map::new();
    assert_eq!(kv.as_hashmap().len(), 0);
}

#[test]
fn test_kvoverwrite_from_hashmap() {
    // Test creation from an existing HashMap
    let mut data = HashMap::new();
    data.insert("key1".to_string(), "value1".to_string());
    data.insert("key2".to_string(), "value2".to_string());

    let kv = Map::from(data.clone());
    assert_eq!(kv.as_hashmap().len(), 2);
    assert_eq!(kv.get("key1"), Some(&"value1".to_string()));
    assert_eq!(kv.get("key2"), Some(&"value2".to_string()));
}

#[test]
fn test_kvoverwrite_remove() {
    // Test removing values
    let mut kv = Map::new();

    // Add a value then remove it
    kv.set("key1", "value1");
    assert_eq!(kv.get("key1"), Some(&"value1".to_string()));

    let removed = kv.remove("key1");
    assert_eq!(removed, Some("value1".to_string()));
    // Assert that key1 is now removed
    assert_eq!(kv.get("key1"), None);

    // Try removing a non-existent key
    let removed = kv.remove("nonexistent");
    assert_eq!(removed, None);
}

#[test]
fn test_kvoverwrite_as_hashmap_mut() {
    // Test mutable access to the underlying HashMap
    let mut kv = Map::new();

    // Modify through the Map methods
    kv.set("key1", "value1");

    // Modify through the mutable HashMap reference
    kv.as_hashmap_mut()
        .insert("key2".to_string(), "value2".to_string());

    // Verify both modifications worked
    assert_eq!(kv.get("key1"), Some(&"value1".to_string()));
    assert_eq!(kv.get("key2"), Some(&"value2".to_string()));
}

#[test]
fn test_kvowrite_to_entry() {
    let mut kvstore = Map::default();
    kvstore.set("key1", "value1");
    kvstore.set("key2", "value2");

    // Serialize the KVOverwrite to a string
    let serialized = serde_json::to_string(&kvstore).unwrap();

    // Create an entry with this data in tmp subtree
    let entry = Entry::root_builder()
        .set_subtree_data("tmp", serialized.clone())
        .build();

    // Ensure the entry data matches the serialized KVOverwrite
    let entry_data = entry.data("tmp").unwrap();
    let deserialized: Map = serde_json::from_str(entry_data).unwrap();

    // Verify the deserialized data matches the original KVOverwrite
    assert_eq!(deserialized.get("key1").unwrap(), "value1");
    assert_eq!(deserialized.get("key2").unwrap(), "value2");
}

#[test]
fn test_kvoverwrite_merge_behavior() {
    // Test merge functionality
    let mut kv = Map::new();

    // Add and then remove some values
    kv.set("key1", "value1");
    kv.set("key2", "value2");

    assert_eq!(kv.get("key1"), Some(&"value1".to_string()));

    // Remove key1, should return the value and actually remove it
    let removed = kv.remove("key1");
    assert_eq!(removed, Some("value1".to_string()));

    // get() should now return None for the removed key
    assert_eq!(kv.get("key1"), None);

    // The key should not exist in the underlying HashMap
    assert!(!kv.as_hashmap().contains_key("key1"));

    // Test merging with new values
    let mut kv2 = Map::new();
    kv2.set("key1", "new_value1"); // Add key1 back
    kv2.set("key3", "value3");

    // Should add the new values
    let merged = kv.merge(&kv2).expect("Merge failed");
    assert_eq!(merged.get("key1"), Some(&"new_value1".to_string())); // Added
    assert_eq!(merged.get("key2"), Some(&"value2".to_string())); // Unchanged
    assert_eq!(merged.get("key3"), Some(&"value3".to_string())); // Added

    // Try removing a non-existent key
    let mut kv3 = Map::new();
    let removed = kv3.remove("nonexistent");
    assert_eq!(removed, None);

    // Merge should preserve existing values
    let final_merge = merged.merge(&kv3).expect("Second merge failed");
    assert_eq!(final_merge.get("key1"), Some(&"new_value1".to_string())); // Still present
    assert_eq!(final_merge.get("key2"), Some(&"value2".to_string())); // Still present
    assert_eq!(final_merge.get("key3"), Some(&"value3".to_string())); // Still present
}

#[test]
fn test_kvoverwrite_serialization_after_remove() {
    // Test serialization after removing values
    let mut kv = Map::new();
    kv.set("key1", "value1");
    kv.set("key2", "value2");

    // Remove a key
    kv.remove("key2");

    // Verify key is removed
    assert!(!kv.as_hashmap().contains_key("key2"));
    assert_eq!(kv.get("key2"), None);

    // Serialize
    let serialized = serde_json::to_string(&kv).expect("Serialization failed");

    // Deserialize
    let deserialized: Map = serde_json::from_str(&serialized).expect("Deserialization failed");

    // Verify structure is maintained
    assert_eq!(deserialized.get("key1"), Some(&"value1".to_string()));
    assert_eq!(deserialized.get("key2"), None);

    // Verify key is still not present
    assert!(!deserialized.as_hashmap().contains_key("key2"));
}

#[test]
fn test_kvoverwrite_remove_nonexistent() {
    // Test removing a key that doesn't exist
    let mut kv = Map::new();

    // Remove a key that doesn't exist
    let result = kv.remove("nonexistent");
    assert_eq!(result, None);

    // Verify no key was created
    assert!(!kv.as_hashmap().contains_key("nonexistent"));

    // Ensure get still returns None
    assert_eq!(kv.get("nonexistent"), None);
}

#[test]
fn test_kvoverwrite_merge_with_removals() {
    // Test merging when both sources have removals
    let mut kv1 = Map::new();
    kv1.set("key1", "value1");
    kv1.set("key2", "value2");
    kv1.remove("key1"); // Remove from kv1

    let mut kv2 = Map::new();
    kv2.set("key2", "updated2");
    kv2.set("key3", "value3");

    // Merge kv2 into kv1
    let merged = kv1.merge(&kv2).expect("Merge failed");

    // Verify results:
    // key1: removed from kv1, not in kv2, so not present
    // key2: value from kv2 overwrites kv1
    // key3: value from kv2

    assert_eq!(merged.get("key1"), None);
    assert_eq!(merged.get("key2"), Some(&"updated2".to_string()));
    assert_eq!(merged.get("key3"), Some(&"value3".to_string()));

    // Verify keys are only present if they have values
    assert!(!merged.as_hashmap().contains_key("key1"));
    assert!(merged.as_hashmap().contains_key("key2"));
    assert!(merged.as_hashmap().contains_key("key3"));
}

#[test]
fn test_kvnested_basic() {
    // Create Nested with string values
    let kv = create_kvnested(&[("key1", "value1"), ("key2", "value2")]);

    // Test get values
    match kv.get("key1") {
        Some(Value::String(s)) => assert_eq!(s, "value1"),
        _ => panic!("Expected string value for key1"),
    }

    match kv.get("key2") {
        Some(Value::String(s)) => assert_eq!(s, "value2"),
        _ => panic!("Expected string value for key2"),
    }

    assert_eq!(kv.get("non_existent"), None);

    // Create a nested map
    let nested = create_nested_kvnested(&[(
        "outer",
        &[("inner1", "nested_value1"), ("inner2", "nested_value2")],
    )]);

    // Test nested access
    assert_nested_value(&nested, &["outer", "inner1"], "nested_value1");
    assert_nested_value(&nested, &["outer", "inner2"], "nested_value2");

    // Test basic merge
    let kv1 = create_kvnested(&[("a", "value_a"), ("b", "value_b")]);
    let kv2 = create_kvnested(&[("b", "updated_b"), ("c", "value_c")]);

    let merged = kv1.merge(&kv2).expect("Merge failed");

    match merged.get("a") {
        Some(Value::String(s)) => assert_eq!(s, "value_a"),
        _ => panic!("Expected string value for merged key a"),
    }

    match merged.get("b") {
        Some(Value::String(s)) => assert_eq!(s, "updated_b"), // Should be updated
        _ => panic!("Expected string value for merged key b"),
    }

    match merged.get("c") {
        Some(Value::String(s)) => assert_eq!(s, "value_c"),
        _ => panic!("Expected string value for merged key c"),
    }
}

#[test]
fn test_kvnested_tombstones() {
    // Create Nested with initial values
    let mut kv = create_kvnested(&[("str_key", "str_value")]);

    // Add a nested map
    let mut nested = Nested::new();
    nested.set_string("inner_key", "inner_value");
    kv.set_map("map_key", nested);

    // Remove a string value
    let removed = kv.remove("str_key");
    match removed {
        Some(Value::String(s)) => assert_eq!(s, "str_value"),
        _ => panic!("Expected to remove a string value"),
    }

    // Verify it's gone from regular access
    assert_eq!(kv.get("str_key"), None);

    // Verify the tombstone using the helper
    assert_path_deleted(&kv, &["str_key"]);

    // Test merging with tombstones
    let kv2 = create_kvnested(&[("str_key", "revived_value")]); // Try to resurrect

    let merged = kv.merge(&kv2).expect("Merge failed");

    // The string should be revived
    match merged.get("str_key") {
        Some(Value::String(s)) => assert_eq!(s, "revived_value"),
        _ => panic!("Expected revived string value"),
    }

    // Now go the other way - delete in kv2 and merge
    let mut kv3 = Nested::new();
    kv3.remove("map_key"); // Delete the map

    let final_merged = merged.merge(&kv3).expect("Second merge failed");

    // The map should be gone - verify using the path helper
    assert_path_deleted(&final_merged, &["map_key"]);

    // But the revived string should remain
    match final_merged.get("str_key") {
        Some(Value::String(s)) => assert_eq!(s, "revived_value"),
        _ => panic!("Expected string value to remain"),
    }
}

#[test]
fn test_kvnested_recursive_merge() {
    // Create two nested structures
    let mut kv1 = Nested::new();

    // Setup level 1
    kv1.set_string("key1", "value1");

    // Setup level 2
    let mut level2 = Nested::new();
    level2.set_string("level2_key1", "level2_value1");
    level2.set_string("shared_key", "kv1_value");

    // Setup level 3
    let mut level3 = Nested::new();
    level3.set_string("level3_key1", "level3_value1");

    // Link them together
    level2.set_map("level3", level3);
    kv1.set_map("level2", level2);

    // Create a second structure with overlapping keys but different values
    let mut kv2 = Nested::new();

    // Setup a different level 2
    let mut level2_alt = Nested::new();
    level2_alt.set_string("level2_key2", "level2_value2");
    level2_alt.set_string("shared_key", "kv2_value"); // Same key, different value

    // Setup a different level 3
    let mut level3_alt = Nested::new();
    level3_alt.set_string("level3_key2", "level3_value2");

    // Link them
    level2_alt.set_map("level3", level3_alt);
    kv2.set_map("level2", level2_alt);

    // Add a top-level key that will conflict
    kv2.set_string("key1", "value1_updated");

    // Merge them
    let merged = kv1.merge(&kv2).expect("Merge failed");

    // Check merged result - top level
    match merged.get("key1") {
        Some(Value::String(s)) => assert_eq!(s, "value1_updated"), // kv2 overwrites
        _ => panic!("Expected updated string at top level"),
    }

    // Level 2 - should contain keys from both sources
    match merged.get("level2") {
        Some(Value::Map(level2_merged)) => {
            // Both unique keys should be present
            match level2_merged.get("level2_key1") {
                Some(Value::String(s)) => assert_eq!(s, "level2_value1"),
                _ => panic!("Expected level2_key1 preserved"),
            }

            match level2_merged.get("level2_key2") {
                Some(Value::String(s)) => assert_eq!(s, "level2_value2"),
                _ => panic!("Expected level2_key2 added"),
            }

            // Shared key should have kv2's value (last write wins)
            match level2_merged.get("shared_key") {
                Some(Value::String(s)) => assert_eq!(s, "kv2_value"),
                _ => panic!("Expected shared_key with kv2's value"),
            }

            // Level 3 - should contain keys from both sources
            match level2_merged.get("level3") {
                Some(Value::Map(level3_merged)) => {
                    match level3_merged.get("level3_key1") {
                        Some(Value::String(s)) => assert_eq!(s, "level3_value1"),
                        _ => panic!("Expected level3_key1 preserved"),
                    }

                    match level3_merged.get("level3_key2") {
                        Some(Value::String(s)) => assert_eq!(s, "level3_value2"),
                        _ => panic!("Expected level3_key2 added"),
                    }
                }
                _ => panic!("Expected merged level3 map"),
            }
        }
        _ => panic!("Expected merged level2 map"),
    }
}

#[test]
fn test_kvnested_serialization() {
    // Test serialization and deserialization of Nested
    let mut kv = Nested::new();

    // Add various value types
    kv.set_string("string_key", "string_value");

    let mut nested = Nested::new();
    nested.set_string("inner", "inner_value");
    kv.set_map("map_key", nested);

    // Create a tombstone
    kv.remove("deleted_key");

    // Serialize
    let serialized = serde_json::to_string(&kv).expect("Serialization failed");

    // Deserialize
    let deserialized: Nested = serde_json::from_str(&serialized).expect("Deserialization failed");

    // Verify string survived
    match deserialized.get("string_key") {
        Some(Value::String(s)) => assert_eq!(s, "string_value"),
        _ => panic!("Expected string value"),
    }

    // Verify nested map survived
    match deserialized.get("map_key") {
        Some(Value::Map(m)) => match m.get("inner") {
            Some(Value::String(s)) => assert_eq!(s, "inner_value"),
            _ => panic!("Expected inner string value"),
        },
        _ => panic!("Expected map value"),
    }

    // Verify tombstone survived
    assert!(deserialized.as_hashmap().contains_key("deleted_key"));
    match deserialized.as_hashmap().get("deleted_key") {
        Some(Value::Deleted) => (),
        _ => panic!("Expected tombstone"),
    }
}

#[test]
fn test_kvnested_cascading_delete() {
    // Test deleting a nested structure
    let mut kv = Nested::new();

    // Create a deeply nested structure
    let mut level1 = Nested::new();
    let mut level2 = Nested::new();
    let mut level3 = Nested::new();

    level3.set_string("deepest", "treasure");
    level2.set_map("level3", level3);
    level1.set_map("level2", level2);
    kv.set_map("level1", level1);

    // Delete the entire structure by removing level1
    kv.remove("level1");

    // Verify it's gone from get
    assert_eq!(kv.get("level1"), None);

    // Verify tombstone exists
    match kv.as_hashmap().get("level1") {
        Some(Value::Deleted) => (),
        _ => panic!("Expected tombstone for level1"),
    }

    // Add a new level1 with different content and verify it works
    let mut new_level1 = Nested::new();
    new_level1.set_string("new_value", "resurrected");
    kv.set_map("level1", new_level1);

    // Verify level1 is accessible again
    match kv.get("level1") {
        Some(Value::Map(m)) => match m.get("new_value") {
            Some(Value::String(s)) => assert_eq!(s, "resurrected"),
            _ => panic!("Expected string in new level1"),
        },
        _ => panic!("Expected map for level1"),
    }
}

#[test]
fn test_kvnested_type_conflicts() {
    // Test merging when same key has different types in different CRDTs
    let mut kv1 = Nested::new();
    let mut kv2 = Nested::new();

    // In kv1, key is a string
    kv1.set_string("conflict_key", "string_value");

    // In kv2, same key is a map
    let mut nested = Nested::new();
    nested.set_string("inner", "inner_value");
    kv2.set_map("conflict_key", nested);

    // Test merge in both directions

    // Direction 1: kv1 -> kv2 (map should win)
    let merged1 = kv1.merge(&kv2).expect("Merge 1 failed");
    match merged1.get("conflict_key") {
        Some(Value::Map(m)) => match m.get("inner") {
            Some(Value::String(s)) => assert_eq!(s, "inner_value"),
            _ => panic!("Expected inner string in map"),
        },
        _ => panic!("Expected map to win in merge 1"),
    }

    // Direction 2: kv2 -> kv1 (map should win)
    let merged2 = kv2.merge(&kv1).expect("Merge 2 failed");
    match merged2.get("conflict_key") {
        Some(Value::String(s)) => assert_eq!(s, "string_value"),
        _ => panic!("Expected string to win in merge 2"),
    }
}

#[test]
fn test_kvnested_complex_merge_with_tombstones() {
    // Test complex merge scenario with multiple levels containing tombstones

    // Structure 1
    let mut kv1 = Nested::new();
    let mut level1a = Nested::new();

    level1a.set_string("key1", "value1");
    level1a.set_string("to_delete", "will_be_deleted");
    level1a.set_string("to_update", "initial_value");

    kv1.set_map("level1", level1a);
    kv1.set_string("top_level_key", "top_value");

    // Structure 2 (with changes and tombstones)
    let mut kv2 = Nested::new();
    let mut level1b = Nested::new();

    level1b.set_string("key2", "value2"); // New key
    level1b.remove("to_delete"); // Create tombstone
    level1b.set_string("to_update", "updated_value"); // Update

    kv2.set_map("level1", level1b);
    kv2.remove("top_level_key"); // Create tombstone at top level
    kv2.set_string("new_top_key", "new_top_value"); // New top level

    // Merge
    let merged = kv1.merge(&kv2).expect("Complex merge failed");

    // Verify top level
    assert_eq!(merged.get("top_level_key"), None); // Should be tombstone
    match merged.get("new_top_key") {
        Some(Value::String(s)) => assert_eq!(s, "new_top_value"),
        _ => panic!("Expected new_top_key"),
    }

    // Verify level1
    match merged.get("level1") {
        Some(Value::Map(level1_merged)) => {
            // Verify level1.key1 (only in kv1, should be preserved)
            match level1_merged.get("key1") {
                Some(Value::String(s)) => assert_eq!(s, "value1"),
                _ => panic!("Expected level1.key1 preserved"),
            }

            // Verify level1.key2 (only in kv2, should be added)
            match level1_merged.get("key2") {
                Some(Value::String(s)) => assert_eq!(s, "value2"),
                _ => panic!("Expected level1.key2 added"),
            }

            // Verify level1.to_delete (deleted in kv2, should be gone)
            assert_eq!(level1_merged.get("to_delete"), None);
            // Verify it's a tombstone
            match level1_merged.as_hashmap().get("to_delete") {
                Some(Value::Deleted) => (),
                _ => panic!("Expected tombstone for level1.to_delete"),
            }

            // Verify level1.to_update (updated in kv2, should have new value)
            match level1_merged.get("to_update") {
                Some(Value::String(s)) => assert_eq!(s, "updated_value"),
                _ => panic!("Expected level1.to_update updated"),
            }
        }
        _ => panic!("Expected level1 map"),
    }
}

#[test]
fn test_kvnested_multi_generation_updates() {
    // Test a sequence of updates and merges to verify LWW semantics

    // Initialize base state
    let mut base = Nested::new();
    base.set_string("key", "original");

    // Generation 1: Update in branch1
    let mut branch1 = Nested::new();
    branch1.set_string("key", "branch1_value");
    let gen1 = base.merge(&branch1).expect("Gen1 merge failed");

    // Verify gen1
    match gen1.get("key") {
        Some(Value::String(s)) => assert_eq!(s, "branch1_value"),
        _ => panic!("Expected branch1 value in gen1"),
    }

    // Generation 2: Delete in branch2
    let mut branch2 = Nested::new();
    branch2.remove("key");
    let gen2 = gen1.merge(&branch2).expect("Gen2 merge failed");

    // Verify gen2
    assert_eq!(gen2.get("key"), None);
    match gen2.as_hashmap().get("key") {
        Some(Value::Deleted) => (),
        _ => panic!("Expected tombstone in gen2"),
    }

    // Generation 3: Resurrect in branch3
    let mut branch3 = Nested::new();
    branch3.set_string("key", "resurrected");
    let gen3 = gen2.merge(&branch3).expect("Gen3 merge failed");

    // Verify gen3
    match gen3.get("key") {
        Some(Value::String(s)) => assert_eq!(s, "resurrected"),
        _ => panic!("Expected resurrected value in gen3"),
    }

    // Generation 4: Replace with map in branch4
    let mut branch4 = Nested::new();
    let mut nested = Nested::new();
    nested.set_string("inner", "inner_value");
    branch4.set_map("key", nested);
    let gen4 = gen3.merge(&branch4).expect("Gen4 merge failed");

    // Verify gen4
    match gen4.get("key") {
        Some(Value::Map(m)) => match m.get("inner") {
            Some(Value::String(s)) => assert_eq!(s, "inner_value"),
            _ => panic!("Expected inner string in gen4"),
        },
        _ => panic!("Expected map in gen4"),
    }
}

#[test]
fn test_kvnested_set_deleted_and_get() {
    let mut kv = Nested::new();

    // Set a key directly to Deleted
    kv.set("deleted_key", Value::Deleted);

    // get() should return None
    assert_eq!(kv.get("deleted_key"), None);

    // as_hashmap() should show the tombstone
    assert_eq!(kv.as_hashmap().get("deleted_key"), Some(&Value::Deleted));

    // Set another key with a value, then set to Deleted
    kv.set_string("another_key", "value");
    kv.set("another_key", Value::Deleted);
    assert_eq!(kv.get("another_key"), None);
    assert_eq!(kv.as_hashmap().get("another_key"), Some(&Value::Deleted));
}

#[test]
fn test_kvnested_remove_non_existent() {
    let mut kv = Nested::new();

    // Remove a key that doesn't exist
    let removed = kv.remove("non_existent_key");
    assert!(
        removed.is_none(),
        "Removing non-existent key should return None"
    );

    // get() should return None
    assert_eq!(kv.get("non_existent_key"), None);

    // as_hashmap() should show a tombstone was created
    assert_eq!(
        kv.as_hashmap().get("non_existent_key"),
        Some(&Value::Deleted)
    );
}

#[test]
fn test_kvnested_remove_existing_tombstone() {
    let mut kv = Nested::new();

    // Create a tombstone by removing a key
    kv.set_string("key_to_tombstone", "some_value");
    let _ = kv.remove("key_to_tombstone"); // This creates the first tombstone

    // Verify it's a tombstone
    assert_eq!(kv.get("key_to_tombstone"), None);
    assert_eq!(
        kv.as_hashmap().get("key_to_tombstone"),
        Some(&Value::Deleted)
    );

    // Try to remove the key again (which is now a tombstone)
    let removed_again = kv.remove("key_to_tombstone");

    // Removing an existing tombstone should return None (as per Nested::remove logic for already deleted)
    assert!(
        removed_again.is_none(),
        "Removing an existing tombstone should return None"
    );

    // get() should still return None
    assert_eq!(kv.get("key_to_tombstone"), None);

    // as_hashmap() should still show the tombstone
    assert_eq!(
        kv.as_hashmap().get("key_to_tombstone"),
        Some(&Value::Deleted)
    );

    // Directly set a tombstone and then remove it
    kv.set("direct_tombstone", Value::Deleted);
    let removed_direct = kv.remove("direct_tombstone");
    assert!(removed_direct.is_none());
    assert_eq!(kv.get("direct_tombstone"), None);
    assert_eq!(
        kv.as_hashmap().get("direct_tombstone"),
        Some(&Value::Deleted)
    );
}

#[test]
fn test_kvnested_merge_dual_tombstones() {
    let mut kv1 = Nested::new();
    kv1.set_string("key1_kv1", "value1_kv1");
    kv1.remove("key1_kv1"); // Tombstone in kv1

    kv1.set_string("common_key", "value_common_kv1");
    kv1.remove("common_key"); // Tombstone for common_key in kv1

    let mut kv2 = Nested::new();
    kv2.set_string("key2_kv2", "value2_kv2");
    kv2.remove("key2_kv2"); // Tombstone in kv2

    kv2.set_string("common_key", "value_common_kv2"); // Value in kv2
    kv2.remove("common_key"); // Tombstone for common_key in kv2 (other's tombstone wins)

    // Merge kv2 into kv1
    let merged = kv1.merge(&kv2).expect("Merge with dual tombstones failed");

    // Check key1_kv1 (only in kv1, tombstoned)
    assert_eq!(merged.get("key1_kv1"), None);
    assert_eq!(merged.as_hashmap().get("key1_kv1"), Some(&Value::Deleted));

    // Check key2_kv2 (only in kv2, tombstoned)
    assert_eq!(merged.get("key2_kv2"), None);
    assert_eq!(merged.as_hashmap().get("key2_kv2"), Some(&Value::Deleted));

    // Check common_key (tombstoned in both, kv2's tombstone should prevail, resulting in a tombstone)
    assert_eq!(merged.get("common_key"), None);
    assert_eq!(merged.as_hashmap().get("common_key"), Some(&Value::Deleted));

    // What if one has a value and the other a tombstone (kv2's tombstone wins)
    let mut kv3 = Nested::new();
    kv3.set_string("val_then_tomb", "i_existed");

    let mut kv4 = Nested::new();
    kv4.remove("val_then_tomb");

    let merged2 = kv3.merge(&kv4).expect("Merge val then tomb failed");
    assert_eq!(merged2.get("val_then_tomb"), None);
    assert_eq!(
        merged2.as_hashmap().get("val_then_tomb"),
        Some(&Value::Deleted)
    );

    // What if one has a tombstone and the other a value (kv4's value wins)
    let merged3 = kv4.merge(&kv3).expect("Merge tomb then val failed");
    match merged3.get("val_then_tomb") {
        Some(Value::String(s)) => assert_eq!(s, "i_existed"),
        _ => panic!("Expected value to overwrite tombstone"),
    }
}

fn setup_kvstore_for_editor_tests(_db: &BaseDB, op: &AtomicOp) -> eidetica::Result<KVStore> {
    op.get_subtree::<KVStore>("my_editor_test_kv_store")
}

#[test]
fn test_value_editor_set_and_get_string_at_root() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_editor_tests(&db, &op)?;

    let editor = store.get_value_mut("user");
    editor.set(Value::String("alice".to_string()))?;

    let retrieved_value = editor.get()?;
    assert_eq!(retrieved_value, Value::String("alice".to_string()));

    // Verify directly from store as well
    assert_eq!(store.get_string("user")?, "alice");

    Ok(())
}

#[test]
fn test_value_editor_set_and_get_nested_string() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_editor_tests(&db, &op)?;

    // Set user.profile.name = "bob"
    let user_editor = store.get_value_mut("user");
    let profile_editor = user_editor.get_value_mut("profile");
    // Get an editor for user.profile.name and set its value
    let name_editor = profile_editor.get_value_mut("name");
    name_editor.set(Value::String("bob".to_string()))?;

    // Get user.profile.name
    let retrieved_name = profile_editor.get_value("name")?;
    assert_eq!(retrieved_name, Value::String("bob".to_string()));

    // Get user.profile (should be a map)
    let profile_map_value = user_editor.get_value("profile")?;
    if let Value::Map(profile_map) = profile_map_value {
        assert_eq!(
            profile_map.get("name"),
            Some(&Value::String("bob".to_string()))
        );
    } else {
        panic!("Expected user.profile to be a map");
    }

    // Get the whole user object
    let user_data = store.get("user")?;
    if let Value::Map(user_map) = user_data {
        if let Some(Value::Map(profile_map)) = user_map.get("profile") {
            assert_eq!(
                profile_map.get("name"),
                Some(&Value::String("bob".to_string()))
            );
        } else {
            panic!("Expected user.profile (nested) to be a map");
        }
    } else {
        panic!("Expected user to be a map");
    }

    Ok(())
}

#[test]
fn test_value_editor_overwrite_non_map_with_map() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_editor_tests(&db, &op)?;

    // Set user = "string_value"
    store.set("user", "string_value")?;

    // Now try to set user.profile.name = "charlie" through editor
    let user_editor = store.get_value_mut("user");
    let profile_editor = user_editor.get_value_mut("profile");
    // Get an editor for user.profile.name and set its value
    let name_editor = profile_editor.get_value_mut("name");
    name_editor.set(Value::String("charlie".to_string()))?;

    // Verify user.profile.name
    // profile_editor.get() should now return the map at "user.profile"
    let profile_value_after_set = profile_editor.get()?;
    if let Value::Map(profile_map_direct) = profile_value_after_set {
        assert_eq!(
            profile_map_direct.get("name"),
            Some(&Value::String("charlie".to_string()))
        );
    } else {
        panic!("Expected profile_editor.get() to be a map");
    }

    let retrieved_name = profile_editor.get_value("name")?;
    assert_eq!(retrieved_name, Value::String("charlie".to_string()));

    // Verify that 'user' is now a map
    let user_data = store.get("user")?;
    assert!(matches!(user_data, Value::Map(_)));
    if let Value::Map(user_map) = user_data {
        if let Some(Value::Map(profile_map)) = user_map.get("profile") {
            assert_eq!(
                profile_map.get("name"),
                Some(&Value::String("charlie".to_string()))
            );
        } else {
            panic!("Expected user.profile to be a map after overwrite");
        }
    } else {
        panic!("Expected user to be a map after overwrite");
    }

    Ok(())
}

#[test]
fn test_value_editor_get_non_existent_path() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_editor_tests(&db, &op)?;

    let editor = store.get_value_mut("nonexistent");
    let result = editor.get();
    assert!(matches!(result, Err(Error::NotFound)));

    let nested_editor = editor.get_value_mut("child");
    let nested_result = nested_editor.get();
    assert!(matches!(nested_result, Err(Error::NotFound)));

    let get_val_result = nested_editor.get_value("grandchild");
    assert!(matches!(get_val_result, Err(Error::NotFound)));

    Ok(())
}

#[test]
fn test_value_editor_set_deeply_nested_creates_path() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_editor_tests(&db, &op)?;

    let editor = store
        .get_value_mut("a")
        .get_value_mut("b")
        .get_value_mut("c");
    editor.set(Value::String("deep_value".to_string()))?;

    // Verify a.b.c = "deep_value"
    let retrieved_value = editor.get()?;
    assert_eq!(retrieved_value, Value::String("deep_value".to_string()));

    let a_val = store.get("a")?;
    if let Value::Map(a_map) = a_val {
        if let Some(Value::Map(b_map)) = a_map.get("b") {
            if let Some(Value::String(c_val)) = b_map.get("c") {
                assert_eq!(c_val, "deep_value");
            } else {
                panic!("Expected a.b.c to be a string");
            }
        } else {
            panic!("Expected a.b to be a map");
        }
    } else {
        panic!("Expected a to be a map");
    }
    Ok(())
}

#[test]
fn test_value_editor_set_string_on_editor_path() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_editor_tests(&db, &op)?;

    let user_editor = store.get_value_mut("user");
    // At this point, user_editor points to ["user"].
    // To make the value at ["user"] be Map({"name": "dave"}), we get an editor for "name" field and set it.
    let name_within_user_editor = user_editor.get_value_mut("name");
    name_within_user_editor.set(Value::String("dave".to_string()))?;

    let user_data = store.get("user")?;
    if let Value::Map(user_map) = user_data {
        assert_eq!(
            user_map.get("name"),
            Some(&Value::String("dave".to_string()))
        );
    } else {
        panic!("Expected user to be a map with name field");
    }

    // Further nesting: user_editor still points to ["user"].
    let profile_editor = user_editor.get_value_mut("profile");
    // profile_editor points to ["user", "profile"].
    // To make value at ["user", "profile"] be Map({"email": ...}), get editor for "email" and set it.
    let email_within_profile_editor = profile_editor.get_value_mut("email");
    email_within_profile_editor.set(Value::String("dave@example.com".to_string()))?;

    let user_data_updated = store.get("user")?;
    if let Value::Map(user_map_updated) = user_data_updated {
        if let Some(Value::Map(profile_map_updated)) = user_map_updated.get("profile") {
            assert_eq!(
                profile_map_updated.get("email"),
                Some(&Value::String("dave@example.com".to_string()))
            );
        } else {
            panic!("Expected user.profile to be a map with email field");
        }
        // Check that "user.name" is still there
        assert_eq!(
            user_map_updated.get("name"),
            Some(&Value::String("dave".to_string()))
        );
    } else {
        panic!("Expected user to be a map after profile update");
    }

    Ok(())
}

// KVStore::get_at_path and KVStore::set_at_path tests

fn setup_kvstore_for_path_tests(op: &AtomicOp) -> eidetica::Result<KVStore> {
    op.get_subtree::<KVStore>("my_path_test_kv_store")
}

#[test]
fn test_kvstore_set_at_path_and_get_at_path_simple() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_path_tests(&op)?;

    let path = ["simple_key"];
    let value = Value::String("simple_value".to_string());

    store.set_at_path(path, value.clone())?;
    let retrieved = store.get_at_path(path)?;
    assert_eq!(retrieved, value);

    // Verify with regular get as well
    assert_eq!(store.get("simple_key")?, value);

    op.commit()?;

    // Verify after commit
    let viewer_op = tree.new_operation()?;
    let viewer_store = setup_kvstore_for_path_tests(&viewer_op)?;
    assert_eq!(viewer_store.get_at_path(path)?, value);
    assert_eq!(viewer_store.get("simple_key")?, value);

    Ok(())
}

#[test]
fn test_kvstore_set_at_path_and_get_at_path_nested() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_path_tests(&op)?;

    let path = ["user", "profile", "email"];
    let value = Value::String("test@example.com".to_string());

    store.set_at_path(path, value.clone())?;
    let retrieved = store.get_at_path(path)?;
    assert_eq!(retrieved, value);

    // Verify intermediate map structure
    let profile_path = ["user", "profile"];
    match store.get_at_path(profile_path)? {
        Value::Map(profile_map) => {
            assert_eq!(profile_map.get("email"), Some(&value));
        }
        _ => panic!("Expected user.profile to be a map"),
    }

    op.commit()?;

    // Verify after commit
    let viewer_op = tree.new_operation()?;
    let viewer_store = setup_kvstore_for_path_tests(&viewer_op)?;
    assert_eq!(viewer_store.get_at_path(path)?, value);

    Ok(())
}

#[test]
fn test_kvstore_set_at_path_creates_intermediate_maps() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_path_tests(&op)?;

    let path = ["a", "b", "c"];
    let value = Value::String("deep_value".to_string());
    store.set_at_path(path, value.clone())?;

    assert_eq!(store.get_at_path(path)?, value);
    match store.get_at_path(["a", "b"])? {
        Value::Map(_) => (),
        _ => panic!("Expected a.b to be a map"),
    }
    match store.get_at_path(["a"])? {
        Value::Map(_) => (),
        _ => panic!("Expected a to be a map"),
    }
    Ok(())
}

#[test]
fn test_kvstore_set_at_path_overwrites_non_map() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_path_tests(&op)?;

    // Set user.profile = "string_value"
    store.set_at_path(
        ["user", "profile"],
        Value::String("string_value".to_string()),
    )?;

    // Now try to set user.profile.name = "charlie"
    let new_path = ["user", "profile", "name"];
    let new_value = Value::String("charlie".to_string());
    store.set_at_path(new_path, new_value.clone())?;

    assert_eq!(store.get_at_path(new_path)?, new_value);

    // Verify that 'user.profile' is now a map
    match store.get_at_path(["user", "profile"])? {
        Value::Map(profile_map) => {
            assert_eq!(profile_map.get("name"), Some(&new_value));
        }
        _ => panic!("Expected user.profile to be a map after overwrite"),
    }
    Ok(())
}

#[test]
fn test_kvstore_get_at_path_not_found() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_path_tests(&op)?;

    let path = ["non", "existent", "key"];
    match store.get_at_path(path) {
        Err(Error::NotFound) => (),
        Ok(v) => panic!("Expected NotFound, got {v:?}"),
        Err(e) => panic!("Expected NotFound, got error {e:?}"),
    }

    // Test path where an intermediate key segment does not exist within a valid map.
    // Set up: existing_root -> some_child_map (empty map)
    let child_map = Nested::new();
    store.set_at_path(["existing_root_map"], Value::Map(child_map))?;

    let path_intermediate_missing = ["existing_root_map", "non_existent_child_in_map", "key"];
    match store.get_at_path(path_intermediate_missing) {
        Err(Error::NotFound) => (),
        Ok(v) => panic!("Expected NotFound for intermediate missing key in map, got {v:?}"),
        Err(e) => panic!("Expected NotFound for intermediate missing key in map, got error {e:?}"),
    }

    // Test path leading to a tombstone
    let tombstone_path = ["deleted", "item"];
    store.set_at_path(tombstone_path, Value::String("temp".to_string()))?;
    store.set_at_path(tombstone_path, Value::Deleted)?;
    match store.get_at_path(tombstone_path) {
        Err(Error::NotFound) => (),
        Ok(v) => panic!("Expected NotFound for tombstone path, got {v:?}"),
        Err(e) => panic!("Expected NotFound for tombstone path, got error {e:?}"),
    }

    Ok(())
}

#[test]
fn test_kvstore_get_at_path_invalid_intermediate_type() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_path_tests(&op)?;

    // Set a.b = "string" (not a map)
    store.set_at_path(["a", "b"], Value::String("i_am_not_a_map".to_string()))?;

    // Try to get a.b.c
    let path = ["a", "b", "c"];
    match store.get_at_path(path) {
        Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::InvalidData => (),
        Ok(v) => panic!("Expected Io(InvalidData), got {v:?}"),
        Err(e) => panic!("Expected Io(InvalidData), got error {e:?}"),
    }
    Ok(())
}

#[test]
fn test_kvstore_set_at_path_empty_path() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_path_tests(&op)?;

    let path: Vec<String> = vec![];

    // Setting a non-map value at the root should fail
    match store.set_at_path(&path, Value::String("test".to_string())) {
        Err(Error::InvalidOperation(_)) => (),
        Ok(_) => panic!("Expected InvalidOperation when setting a non-map at root"),
        Err(e) => panic!("Expected InvalidOperation, got error {e:?}"),
    }

    // Setting a map value at the root should succeed
    let nested_map = Nested::new();
    match store.set_at_path(&path, Value::Map(nested_map)) {
        Ok(_) => (),
        Err(e) => panic!("Expected success when setting map at root, got error {e:?}"),
    }

    Ok(())
}

#[test]
fn test_kvstore_get_at_path_empty_path() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_path_tests(&op)?;

    let path: Vec<String> = vec![];

    // Getting the root should return a map (the entire KVStore contents)
    match store.get_at_path(&path) {
        Ok(Value::Map(_)) => (),
        Ok(v) => panic!("Expected Map for root path, got {v:?}"),
        Err(e) => panic!("Expected success for root path, got error {e:?}"),
    }

    Ok(())
}

#[test]
fn test_value_editor_root_operations() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_path_tests(&op)?;

    // Set some values at the top level
    store.set("key1", "value1")?;
    store.set("key2", "value2")?;

    // Get a root editor
    let root_editor = store.get_root_mut();

    // We should be able to get values via the root editor
    match root_editor.get()? {
        Value::Map(map) => {
            let entries = map.as_hashmap();
            assert!(entries.contains_key("key1"));
            assert!(entries.contains_key("key2"));
        }
        _ => panic!("Expected root editor to get a map"),
    }

    // Get values directly from the top level
    match root_editor.get_value("key1")? {
        Value::String(s) => assert_eq!(s, "value1"),
        _ => panic!("Expected string value"),
    }

    // Create a new nested map at root level
    let mut nested = Nested::new();
    nested.set_string("nested_key", "nested_value");
    root_editor
        .get_value_mut("nested")
        .set(Value::Map(nested))?;

    // Verify the nested structure
    match root_editor.get_value("nested")? {
        Value::Map(map) => {
            let entries = map.as_hashmap();
            assert!(entries.contains_key("nested_key"));
        }
        _ => panic!("Expected nested map"),
    }

    // Delete a value at root level
    root_editor.delete_child("key1")?;

    // Verify deletion
    match root_editor.get_value("key1") {
        Err(Error::NotFound) => (),
        Ok(v) => panic!("Expected NotFound after deletion, got {v:?}"),
        Err(e) => panic!("Expected NotFound after deletion, got error {e:?}"),
    }

    op.commit()?;

    // Verify after commit
    let viewer_op = tree.new_operation()?;
    let viewer_store = setup_kvstore_for_path_tests(&viewer_op)?;
    match viewer_store.get("key1") {
        Err(Error::NotFound) => (),
        Ok(v) => panic!("Expected NotFound after commit, got {v:?}"),
        Err(e) => panic!("Expected NotFound after commit, got error {e:?}"),
    }

    assert_eq!(viewer_store.get_string("key2")?, "value2");

    Ok(())
}

#[test]
fn test_value_editor_delete_methods() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_editor_tests(&db, &op)?;

    // Set up a nested structure
    let mut user_profile = Nested::new();
    user_profile.set_string("name", "Alice");
    user_profile.set_string("email", "alice@example.com");

    let mut user_data = Nested::new();
    user_data.set("profile", Value::Map(user_profile));
    user_data.set_string("role", "admin");

    store.set_value("user", Value::Map(user_data))?;

    // Get an editor for the user object
    let user_editor = store.get_value_mut("user");

    // Test delete_child method
    user_editor.delete_child("role")?;

    // Verify the role is deleted
    match user_editor.get_value("role") {
        Err(Error::NotFound) => (),
        Ok(v) => panic!("Expected NotFound after delete_child, got {v:?}"),
        Err(e) => panic!("Expected NotFound after delete_child, got error {e:?}"),
    }

    // The profile should still exist
    match user_editor.get_value("profile")? {
        Value::Map(_) => (),
        _ => panic!("Expected profile map to still exist"),
    }

    // Get editor for profile
    let profile_editor = user_editor.get_value_mut("profile");

    // Test delete_self method
    profile_editor.delete_self()?;

    // Verify the profile is deleted
    match user_editor.get_value("profile") {
        Err(Error::NotFound) => (),
        Ok(v) => panic!("Expected NotFound after delete_self, got {v:?}"),
        Err(e) => panic!("Expected NotFound after delete_self, got error {e:?}"),
    }

    // But the parent object (user) should still exist
    match store.get("user")? {
        Value::Map(_) => (),
        _ => panic!("Expected user map to still exist"),
    }

    op.commit()?;

    // Verify after commit
    let viewer_op = tree.new_operation()?;
    let viewer_store = setup_kvstore_for_editor_tests(&db, &viewer_op)?;

    // User exists but has no role or profile
    match viewer_store.get("user")? {
        Value::Map(map) => {
            let entries = map.as_hashmap();

            // Check that the entries are properly marked as deleted (tombstones)
            match entries.get("role") {
                Some(Value::Deleted) => (),
                Some(other) => panic!("Expected role to be deleted, got {other:?}"),
                None => panic!("Expected role key with tombstone to exist"),
            }

            match entries.get("profile") {
                Some(Value::Deleted) => (),
                Some(other) => panic!("Expected profile to be deleted, got {other:?}"),
                None => panic!("Expected profile key with tombstone to exist"),
            }
        }
        _ => panic!("Expected user to be a map after commit"),
    }

    Ok(())
}

#[test]
fn test_value_editor_set_non_map_to_root() -> eidetica::Result<()> {
    let db = BaseDB::new(Box::new(InMemory::new()));
    db.add_private_key("test_key")?;
    let tree = db.new_tree_default("test_key")?;
    let op = tree.new_operation()?;
    let store = setup_kvstore_for_path_tests(&op)?;

    // Get a root editor
    let root_editor = store.get_root_mut();

    // Attempting to set a non-map value at root should fail
    let result = root_editor.set(Value::String("test string".to_string()));

    // Check that we get an InvalidOperation error
    match result {
        Err(Error::InvalidOperation(_)) => (),
        Ok(_) => panic!("Expected InvalidOperation error when setting non-map at root"),
        Err(e) => panic!("Expected InvalidOperation, got error: {e:?}"),
    }

    // Setting a map value should succeed
    let mut map = Nested::new();
    map.set_string("key", "value");
    let map_result = root_editor.set(Value::Map(map));
    assert!(map_result.is_ok());

    Ok(())
}
