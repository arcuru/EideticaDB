use eidetica::constants::ROOT;
use eidetica::entry::{Entry, ID};

#[test]
fn test_entry_creation() {
    let root = "test_root";
    let entry = Entry::builder(root).build();

    assert_eq!(entry.root(), root);
    assert!(!entry.is_root()); // Regular entries are not root entries
    assert!(!entry.is_toplevel_root()); // Should be false as it's not a top-level entry

    assert!(entry.parents().unwrap().is_empty()); // New entry has no parents
}

#[test]
fn test_entry_toplevel_creation() {
    let entry = Entry::root_builder().build();

    assert!(entry.root().is_empty());
    assert!(entry.is_root());
    assert!(entry.in_subtree(ROOT)); // Top-level entries have a "root" subtree
}

#[test]
fn test_entry_add_subtree() {
    let root = "test_root_parents";

    // Part 1: Create entry using the builder pattern directly
    let subtree_name = "subtree1";
    let subtree_data = "subtree_data";

    // Use the builder pattern with direct chaining (no variable)
    let entry = Entry::builder(root)
        .set_subtree_data(subtree_name, subtree_data)
        .build();

    // Verify subtree was added
    let subtrees = entry.subtrees();
    assert_eq!(subtrees.len(), 1);
    assert_eq!(subtrees[0], subtree_name);

    // Verify subtree data
    let fetched_data = entry.data(subtree_name).unwrap();
    assert_eq!(fetched_data, subtree_data);

    // Check subtree parents
    let subtree_parents = entry.subtree_parents(subtree_name).unwrap();
    assert!(subtree_parents.is_empty()); // New subtree has no parents initially

    // Part 2: Test overwrite using the mutable reference pattern
    let mut builder = Entry::builder(root);
    builder.set_subtree_data_mut(subtree_name, subtree_data);
    let new_subtree_data = "new_subtree_data";
    builder.set_subtree_data_mut(subtree_name, new_subtree_data);

    // Build the entry
    let new_entry = builder.build();

    // Verify count is still 1
    assert_eq!(new_entry.subtrees().len(), 1);

    // Verify data was overwritten
    let fetched_new_data = new_entry.data(subtree_name).unwrap();
    assert_eq!(fetched_new_data, new_subtree_data);
}

#[test]
fn test_entry_parents() {
    let root = "test_root_parents";
    let mut builder = Entry::builder(root);

    // Set parents for the main tree
    let parent1: ID = "parent1".into();
    let parent2: ID = "parent2".into();
    let parents = vec![parent1.clone(), parent2.clone()];
    builder.set_parents_mut(parents.clone());

    // Add a subtree
    let subtree_name = "subtree1";
    let subtree_data = "subtree_data";
    builder.set_subtree_data_mut(subtree_name, subtree_data);

    // Set subtree parents
    let subtree_parent: ID = "subtree_parent".into();
    builder.set_subtree_parents_mut(subtree_name, vec![subtree_parent.clone()]);

    // Build the entry
    let entry = builder.build();

    // Verify parents were set
    let fetched_parents = entry.parents().unwrap();
    assert_eq!(fetched_parents, parents);

    // Verify subtree parents
    let fetched_subtree_parents = entry.subtree_parents(subtree_name).unwrap();
    assert_eq!(fetched_subtree_parents, vec![subtree_parent]);
}

#[test]
fn test_entry_id() {
    let root = "test_root_id";
    let entry = Entry::builder(root).build();

    let id = entry.id();
    assert!(!id.is_empty());

    // Create identical entry - should have same ID (content-addressable)
    let identical_entry = Entry::builder(root).build();
    assert_eq!(identical_entry.id(), id);

    // Create different entry - should have different ID
    let different_entry = Entry::builder("different_root").build();
    assert_ne!(different_entry.id(), id);
}

#[test]
fn test_in_tree_and_subtree() {
    let root = "test_root_subtrees";
    let mut builder = Entry::builder(root);

    let subtree_name = "subtree1";
    builder.set_subtree_data_mut(subtree_name, "subtree_data");

    let entry = builder.build();

    assert!(entry.in_tree(root));
    assert!(!entry.in_tree("other_tree"));
    assert!(entry.in_subtree(subtree_name));
    assert!(!entry.in_subtree("non_existent_subtree"));
}

#[test]
fn test_entry_with_multiple_subtrees() {
    let root = "test_root_order";

    // Create a builder
    let mut builder = Entry::builder(root);

    // Add several subtrees
    let subtrees = [
        ("users", "user_data"),
        ("posts", "post_data"),
        ("comments", "comment_data"),
        ("ratings", "rating_data"),
    ];

    for (name, data) in subtrees.iter() {
        builder.set_subtree_data_mut(*name, *data);
    }

    // Add parents to each subtree
    for (name, _) in subtrees.iter() {
        let parent_id: ID = format!("parent_for_{name}").into();
        builder.set_subtree_parents_mut(*name, vec![parent_id.clone()]);
    }

    // Build the entry
    let entry = builder.build();

    // Verify all subtrees were added
    let subtree_names = entry.subtrees();
    assert_eq!(subtree_names.len(), 4);

    // Verify each subtree has the right data
    for (name, data) in subtrees.iter() {
        assert!(entry.in_subtree(name));
        assert_eq!(entry.data(name).unwrap(), data);
    }

    // Try to access a non-existent subtree
    let non_existent = entry.data("non_existent");
    assert!(non_existent.is_err());

    // Verify parents were set correctly
    for (name, _) in subtrees.iter() {
        let parent_id = format!("parent_for_{name}");
        let parents = entry.subtree_parents(name).unwrap();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0], parent_id);
    }
}

#[test]
fn test_entry_id_determinism() {
    // Test that entries with the same data but created differently have the same ID

    // First entry
    let mut builder1 = Entry::builder("test_root");
    // Parents order should not matter
    builder1.set_parents_mut(vec!["parent1".into(), "parent2".into()]);
    builder1.set_subtree_data_mut("subtree1", "data1");
    builder1.set_subtree_data_mut("subtree2", "data2");
    builder1.set_subtree_parents_mut("subtree1", vec!["sub_parent1".into()]);
    let entry1 = builder1.build();

    // Second entry with same content but adding subtrees and parents in different order
    let mut builder2 = Entry::builder("test_root");
    // Order of adding subtrees should not matter
    builder2.set_subtree_data_mut("subtree2", "data2");
    builder2.set_subtree_data_mut("subtree1", "data1");
    // Order of parents should not matter
    // Now using different order to test that the order of parents does not matter
    builder2.set_parents_mut(vec!["parent2".into(), "parent1".into()]);
    builder2.set_subtree_parents_mut("subtree1", vec!["sub_parent1".into()]);
    let entry2 = builder2.build();

    // IDs should be the same
    assert_eq!(entry1.id(), entry2.id());

    // Now modify entry2 in a subtle way
    let mut builder3 = Entry::builder("test_root");
    builder3.set_parents_mut(vec!["parent2".into(), "parent1".into()]);
    builder3.set_subtree_data_mut("subtree2", "data2");
    builder3.set_subtree_data_mut("subtree1", "data1");
    builder3.set_subtree_parents_mut("subtree1", vec!["different_parent".into()]);
    let entry3 = builder3.build();

    // IDs should now be different
    assert_ne!(entry1.id(), entry3.id());
}

#[test]
fn test_entry_remove_empty_subtrees() {
    let root = "test_root_build";
    let mut builder = Entry::builder(root);

    // Add some subtrees, some with data, some without
    builder.set_subtree_data_mut("sub1", "data1");
    builder.set_subtree_data_mut("sub2_empty", ""); // Empty data
    builder.set_subtree_data_mut("sub3", "data3");

    // Apply remove_empty_subtrees
    builder.remove_empty_subtrees_mut();

    // Build the entry
    let entry = builder.build();

    // Verify empty subtree was removed
    let remaining_subtrees = entry.subtrees();
    assert_eq!(remaining_subtrees.len(), 2);
    assert!(remaining_subtrees.contains(&"sub1".to_string()));
    assert!(remaining_subtrees.contains(&"sub3".to_string()));
    assert!(!remaining_subtrees.contains(&"sub2_empty".to_string()));

    // Verify data of remaining subtrees is intact
    assert_eq!(entry.data("sub1").unwrap(), "data1");
    assert_eq!(entry.data("sub3").unwrap(), "data3");
}

#[test]
fn test_add_subtree_success() {
    let mut builder = Entry::builder("root_id");
    builder.set_subtree_data_mut("test", "{}");
    let entry = builder.build();

    // Verify the subtree exists
    assert!(entry.in_subtree("test"));
}

#[test]
fn test_add_subtree_duplicate() {
    let mut builder = Entry::builder("root_id");

    // Add the subtree twice
    builder.set_subtree_data_mut("test", "{}");
    builder.set_subtree_data_mut("test", "{}");

    let entry = builder.build();

    // Verify there is only one subtree
    assert_eq!(entry.subtrees().len(), 1);
}

#[test]
fn test_subtrees_are_sorted() {
    let mut builder = Entry::builder("root_id");

    // Add subtrees out of order
    builder.set_subtree_data_mut("c", "{}");
    builder.set_subtree_data_mut("a", "{}");
    builder.set_subtree_data_mut("b", "{}");

    let entry = builder.build();

    // Verify subtrees are sorted alphabetically
    let subtrees = entry.subtrees();
    assert_eq!(
        subtrees,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[test]
fn test_parents_are_sorted() {
    let mut builder = Entry::builder("root_id");

    // Add parents out of order
    builder.set_parents_mut(vec!["c".into(), "a".into(), "b".into()]);

    // Add a subtree with parents out of order
    builder.set_subtree_data_mut("test", "{}");
    builder.set_subtree_parents_mut("test", vec!["z".into(), "x".into(), "y".into()]);

    let entry = builder.build();

    // Verify main tree parents are sorted
    let main_parents = entry.parents().unwrap();
    assert_eq!(
        main_parents,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );

    // Verify subtree parents are sorted
    let subtree_parents = entry.subtree_parents("test").unwrap();
    assert_eq!(
        subtree_parents,
        vec!["x".to_string(), "y".to_string(), "z".to_string()]
    );
}

#[test]
fn test_dual_api_patterns() {
    // Test 1: Builder pattern with ownership
    // This pattern takes self and returns Self, allowing method chaining
    let entry = Entry::builder("root_id")
        .set_parents(vec!["parent1".into(), "parent2".into()])
        .set_subtree_data("subtree1", "subtree_data1")
        .set_subtree_parents("subtree1", vec!["subtree_parent1".into()])
        .add_subtree_parent("subtree1", "subtree_parent2")
        .build();

    // Verify the entry was built correctly
    assert_eq!(entry.root(), "root_id");
    assert!(entry.in_subtree("subtree1"));
    assert_eq!(entry.data("subtree1").unwrap(), "subtree_data1");

    let parents = entry.parents().unwrap();
    assert_eq!(parents.len(), 2);
    assert!(parents.contains(&"parent1".into()));
    assert!(parents.contains(&"parent2".into()));

    let subtree_parents = entry.subtree_parents("subtree1").unwrap();
    assert_eq!(subtree_parents.len(), 2);
    assert!(subtree_parents.contains(&"subtree_parent1".into()));
    assert!(subtree_parents.contains(&"subtree_parent2".into()));

    // Test 2: Mutable reference pattern
    // This pattern takes &mut self and returns &mut Self
    // Useful when you need to keep the builder in a variable
    let mut builder = Entry::builder("root_id2");

    // Use the _mut methods for modifications
    builder
        .set_parents_mut(vec!["parent3".into(), "parent4".into()])
        .set_subtree_data_mut("subtree2", "subtree_data2")
        .set_subtree_parents_mut("subtree2", vec!["subtree_parent3".into()])
        .add_subtree_parent_mut("subtree2", "subtree_parent4");

    // Make additional modifications
    // Note: set_data_mut was removed since TreeNode.data is no longer used

    // Build the entry
    let entry2 = builder.build();

    // Verify the entry was built correctly
    assert_eq!(entry2.root(), "root_id2");
    assert!(entry2.in_subtree("subtree2"));
    assert_eq!(entry2.data("subtree2").unwrap(), "subtree_data2");

    let parents2 = entry2.parents().unwrap();
    assert_eq!(parents2.len(), 2);
    assert!(parents2.contains(&"parent3".into()));
    assert!(parents2.contains(&"parent4".into()));

    let subtree_parents2 = entry2.subtree_parents("subtree2").unwrap();
    assert_eq!(subtree_parents2.len(), 2);
    assert!(subtree_parents2.contains(&"subtree_parent3".into()));
    assert!(subtree_parents2.contains(&"subtree_parent4".into()));
}

#[test]
fn test_entrybuilder_api_consistency() {
    // Test that both ownership and mutable reference APIs produce identical results

    // First entry using ownership chaining API
    let entry1 = Entry::builder("root")
        .set_parents(vec!["parent1".into(), "parent2".into()])
        .set_subtree_data("subtree1", "data1")
        .set_subtree_parents("subtree1", vec!["sp1".into()])
        .add_parent("parent3")
        .add_subtree_parent("subtree1", "sp2")
        .remove_empty_subtrees()
        .build();

    // Second entry using mutable reference API
    let mut builder2 = Entry::builder("root");
    builder2
        .set_parents_mut(vec!["parent1".into(), "parent2".into()])
        .set_subtree_data_mut("subtree1", "data1")
        .set_subtree_parents_mut("subtree1", vec!["sp1".into()])
        .add_parent_mut("parent3")
        .add_subtree_parent_mut("subtree1", "sp2")
        .remove_empty_subtrees_mut();
    let entry2 = builder2.build();

    // IDs should be identical, showing that both APIs produce equivalent results
    assert_eq!(entry1.id(), entry2.id());
}

#[test]
fn test_entrybuilder_empty_subtree_removal() {
    // Test the behavior of removing empty subtrees

    // Create a builder with one subtree with data and one with empty data
    let builder = Entry::builder("root")
        .set_subtree_data("subtree1", "data1")
        .set_subtree_data("empty", "");

    // Create two copies to test each API
    let entry1 = builder.clone().remove_empty_subtrees().build();

    let mut builder2 = builder.clone();
    builder2.remove_empty_subtrees_mut();
    let entry2 = builder2.build();

    // Both entries should have only one subtree (the empty one should be removed)
    assert_eq!(entry1.subtrees().len(), 1);
    assert_eq!(entry2.subtrees().len(), 1);

    // Both should have the same ID
    assert_eq!(entry1.id(), entry2.id());

    // Both should have the non-empty subtree
    assert!(entry1.in_subtree("subtree1"));
    assert!(!entry1.in_subtree("empty"));
}

#[test]
fn test_entrybuilder_parent_deduplication() {
    // Test that duplicate parent IDs are handled correctly

    // Create an entry with duplicate parents in both main tree and subtree
    let entry = Entry::builder("test_root")
        .set_parents(vec![
            "parent1".into(),
            "parent2".into(),
            "parent1".into(), // Duplicate
        ])
        .set_subtree_data("subtree1", "data1")
        .set_subtree_parents(
            "subtree1",
            vec![
                "sp1".into(),
                "sp2".into(),
                "sp1".into(), // Duplicate
            ],
        )
        .build();

    // Check that the main tree parents have duplicates removed
    let tree_parents = entry.parents().unwrap();
    assert_eq!(tree_parents.len(), 2);
    assert!(tree_parents.contains(&"parent1".into()));
    assert!(tree_parents.contains(&"parent2".into()));

    // Check that the subtree parents have duplicates removed
    let subtree_parents = entry.subtree_parents("subtree1").unwrap();
    assert_eq!(subtree_parents.len(), 2);
    assert!(subtree_parents.contains(&"sp1".into()));
    assert!(subtree_parents.contains(&"sp2".into()));
}

#[test]
fn test_entrybuilder_id_stability() {
    // Test that Entry IDs are consistent regardless of insertion order

    // First entry with parents and subtrees added in one order
    let entry1 = Entry::builder("test_root")
        .set_parents(vec!["parent1".into(), "parent2".into()])
        .set_subtree_data("subtree1", "data1")
        .set_subtree_data("subtree2", "data2")
        .set_subtree_parents("subtree1", vec!["sp1".into()])
        .build();

    // Second entry with identical content but added in reverse order
    let entry2 = Entry::builder("test_root")
        .set_parents(vec!["parent2".into(), "parent1".into()]) // Reversed
        .set_subtree_data("subtree2", "data2") // Reversed
        .set_subtree_data("subtree1", "data1")
        .set_subtree_parents("subtree1", vec!["sp1".into()])
        .build();

    // Third entry with the same content but subtree parents set after subtree data
    let entry3 = Entry::builder("test_root")
        .set_subtree_data("subtree1", "data1")
        .set_subtree_data("subtree2", "data2")
        .set_parents(vec!["parent1".into(), "parent2".into()])
        .set_subtree_parents("subtree1", vec!["sp1".into()])
        .build();

    // All three entries should have the same ID
    assert_eq!(entry1.id(), entry2.id());
    assert_eq!(entry2.id(), entry3.id());
}

#[test]
fn test_entrybuilder_edge_cases() {
    // Test behavior of EntryBuilder with edge cases

    // Empty builder
    let empty_builder = Entry::builder(String::new());
    let empty_entry = empty_builder.build();
    assert_eq!(empty_entry.root(), ""); // Default root should be empty string
    assert!(empty_entry.parents().unwrap().is_empty()); // No parents
    assert!(empty_entry.subtrees().is_empty()); // No subtrees

    // Builder with empty subtree names
    let entry_with_empty_subtree = Entry::builder("test_root")
        .set_subtree_data("", "empty_subtree_data")
        .build();

    // Verify the empty-named subtree exists
    assert!(entry_with_empty_subtree.in_subtree(""));
    assert_eq!(
        entry_with_empty_subtree.data("").unwrap(),
        "empty_subtree_data"
    );

    // Builder with a subtree overriding the root ID
    let root_override = Entry::builder("test_root")
        .set_subtree_data(ROOT, "root_data")
        .build();

    // This should create a subtree named ROOT, not change the root ID
    assert!(root_override.in_subtree(ROOT));
    assert_eq!(root_override.data(ROOT).unwrap(), "root_data");
    assert_eq!(root_override.root(), "test_root"); // Root ID is still "test_root"
}

#[test]
fn test_entrybuilder_add_parent_methods() {
    // Test the add_parent and add_parent_mut methods

    // Start with no parents
    let mut builder = Entry::builder("test_root");

    // Add first parent with mutable method
    builder.add_parent_mut("parent1");

    // Add second parent with ownership method
    let builder = builder.add_parent("parent2");

    // Build the entry
    let entry = builder.build();

    // Check that both parents were added
    let parents = entry.parents().unwrap();
    assert_eq!(parents.len(), 2);
    assert!(parents.contains(&"parent1".into()));
    assert!(parents.contains(&"parent2".into()));

    // Also test adding to an existing list of parents
    let entry2 = Entry::builder("test_root")
        .set_parents(vec!["parent1".into(), "parent2".into()])
        .add_parent("parent3")
        .build();

    let parents2 = entry2.parents().unwrap();
    assert_eq!(parents2.len(), 3);
    assert!(parents2.contains(&"parent3".into()));
}

#[test]
fn test_entrybuilder_subtree_parent_methods() {
    // Test the add_subtree_parent and add_subtree_parent_mut methods

    // Create a builder with a subtree
    let mut builder = Entry::builder("test_root").set_subtree_data("subtree1", "data1");

    // Add first subtree parent with mutable method
    builder.add_subtree_parent_mut("subtree1", "sp1");

    // Add second subtree parent with ownership method
    let builder = builder.add_subtree_parent("subtree1", "sp2");

    // Build the entry
    let entry = builder.build();

    // Check that both subtree parents were added
    let subtree_parents = entry.subtree_parents("subtree1").unwrap();
    assert_eq!(subtree_parents.len(), 2);
    assert!(subtree_parents.contains(&"sp1".into()));
    assert!(subtree_parents.contains(&"sp2".into()));

    // Also test adding to an existing list of subtree parents
    let entry2 = Entry::builder("test_root")
        .set_subtree_data("subtree1", "data1")
        .set_subtree_parents("subtree1", vec!["sp1".into(), "sp2".into()])
        .add_subtree_parent("subtree1", "sp3")
        .build();

    let subtree_parents2 = entry2.subtree_parents("subtree1").unwrap();
    assert_eq!(subtree_parents2.len(), 3);
    assert!(subtree_parents2.contains(&"sp3".into()));

    // Test adding a parent to a non-existent subtree (should create the subtree)
    let entry3 = Entry::builder("test_root")
        .add_subtree_parent("new_subtree", "sp1")
        .build();

    assert!(entry3.in_subtree("new_subtree"));
    let new_subtree_parents = entry3.subtree_parents("new_subtree").unwrap();
    assert_eq!(new_subtree_parents.len(), 1);
    assert_eq!(new_subtree_parents[0], "sp1");
}
