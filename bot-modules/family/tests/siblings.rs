//! Pure sibling-derivation logic (`collect_sibling_ids`): a user's siblings are
//! their parents' children minus themselves, unioned across multiple parents.

use family::FamilyRow;
use family::commands::collect_sibling_ids;

fn row_with_children(id: i64, children: Vec<i64>) -> FamilyRow {
    FamilyRow { id, children_ids: children, ..Default::default() }
}

#[test]
fn collect_sibling_ids_combines_children_from_multiple_parents() {
    let user_id = 1;
    let parent_a = row_with_children(10, vec![user_id, 2]);
    let parent_b = row_with_children(20, vec![user_id, 3]);

    let siblings = collect_sibling_ids(&[parent_a, parent_b], user_id);

    assert_eq!(siblings, vec![2, 3]);
}

#[test]
fn collect_sibling_ids_excludes_only_self() {
    let user_id = 1;
    let parent = row_with_children(10, vec![user_id]);

    let siblings = collect_sibling_ids(&[parent], user_id);

    assert_eq!(siblings, Vec::<i64>::new());
}
