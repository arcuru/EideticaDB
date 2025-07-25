use super::helpers::*;
use eidetica::entry::Entry;

#[test]
fn test_entry_id_determinism() {
    // Test that entries with the same data but created differently have the same ID

    // First entry - using helper for simple parts, builder for complex
    let mut builder1 = Entry::builder("test_root");
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
    assert_same_id(&entry1, &entry2);

    // Now modify entry2 in a subtle way
    let mut builder3 = Entry::builder("test_root");
    builder3.set_parents_mut(vec!["parent2".into(), "parent1".into()]);
    builder3.set_subtree_data_mut("subtree2", "data2");
    builder3.set_subtree_data_mut("subtree1", "data1");
    builder3.set_subtree_parents_mut("subtree1", vec!["different_parent".into()]);
    let entry3 = builder3.build();

    // IDs should now be different
    assert_different_id(&entry1, &entry3);
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
    assert_same_id(&entry1, &entry2);
    assert_same_id(&entry2, &entry3);
}

#[test]
fn test_id_determinism_with_complex_structure() {
    // Test determinism with a more complex entry structure

    let entry_a = create_complex_entry_with_order("complex_root", false);
    let entry_b = create_complex_entry_with_order("complex_root", true);

    // Should have the same ID despite different construction order
    assert_same_id(&entry_a, &entry_b);
}
