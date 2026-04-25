use std::collections::BTreeSet;

use crate::{collider::ShapeAabb, handles::ColliderHandle, math::FloatNum};

const FAT_AABB_MIN_MARGIN: FloatNum = 0.1;
const FAT_AABB_EXTENT_RATIO: FloatNum = 0.1;
const MIN_REBUILD_LEAF_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ColliderProxy {
    pub(crate) handle: ColliderHandle,
    pub(crate) aabb: ShapeAabb,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BroadphaseStats {
    pub(crate) candidate_count: usize,
    pub(crate) update_count: usize,
    pub(crate) stale_proxy_drop_count: usize,
    pub(crate) same_body_drop_count: usize,
    pub(crate) filter_drop_count: usize,
    pub(crate) narrowphase_drop_count: usize,
    pub(crate) rebuild_count: usize,
    pub(crate) tree_depth: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BroadphaseOutput {
    pub(crate) candidate_pairs: Vec<(usize, usize)>,
    pub(crate) stats: BroadphaseStats,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Broadphase {
    tree: DynamicAabbTree,
}

impl Broadphase {
    pub(crate) fn update(&mut self, proxies: &[ColliderProxy]) -> BroadphaseOutput {
        let live_handles = proxies
            .iter()
            .map(|proxy| proxy.handle)
            .collect::<BTreeSet<_>>();
        let mut update_count = 0;
        let mut stale_proxy_drop_count = 0;

        for stale in self.tree.handles() {
            if !live_handles.contains(&stale) && self.tree.remove_proxy(stale) {
                update_count += 1;
                stale_proxy_drop_count += 1;
            }
        }

        for (proxy_index, proxy) in proxies.iter().copied().enumerate() {
            if self.tree.update_proxy(proxy_index, proxy) {
                update_count += 1;
            }
        }

        let mut rebuild_count = 0;
        if self.tree.needs_rebuild() || self.tree.needs_compaction() {
            self.tree.rebuild_balanced_from_proxies(proxies);
            rebuild_count = 1;
        }

        let candidate_pairs = self.tree.candidate_pairs();
        BroadphaseOutput {
            stats: BroadphaseStats {
                candidate_count: candidate_pairs.len(),
                update_count,
                stale_proxy_drop_count,
                same_body_drop_count: 0,
                filter_drop_count: 0,
                narrowphase_drop_count: 0,
                rebuild_count,
                tree_depth: self.tree.depth(),
            },
            candidate_pairs,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DynamicAabbTree {
    nodes: Vec<TreeNode>,
    root: Option<usize>,
    leaves: Vec<usize>,
}

#[derive(Clone, Debug)]
struct TreeNode {
    aabb: ShapeAabb,
    parent: Option<usize>,
    left: Option<usize>,
    right: Option<usize>,
    proxy_index: Option<usize>,
    handle: ColliderHandle,
}

impl DynamicAabbTree {
    #[cfg(test)]
    pub(crate) fn from_proxies(proxies: &[ColliderProxy]) -> Self {
        let mut tree = Self::default();
        for (proxy_index, proxy) in proxies.iter().enumerate() {
            tree.insert_leaf(proxy_index, *proxy);
        }
        tree
    }

    pub(crate) fn candidate_pairs(&self) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();

        for &leaf_index in &self.leaves {
            let leaf = &self.nodes[leaf_index];
            let Some(proxy_index) = leaf.proxy_index else {
                continue;
            };
            self.query_overlaps(leaf_index, leaf.aabb, |other_leaf_index| {
                let other = &self.nodes[other_leaf_index];
                let Some(other_proxy_index) = other.proxy_index else {
                    return;
                };
                if proxy_index >= other_proxy_index {
                    return;
                }
                pairs.push((proxy_index, other_proxy_index));
            });
        }

        // Preserve the old contact pass ordering: live collider snapshot order,
        // not handle ordering. Recycled handles may compare differently because
        // generation bits participate in `Ord`.
        pairs.sort_unstable();
        pairs
    }

    fn handles(&self) -> Vec<ColliderHandle> {
        self.leaves
            .iter()
            .filter_map(|&leaf_index| {
                self.nodes
                    .get(leaf_index)
                    .and_then(|node| node.proxy_index.map(|_| node.handle))
            })
            .collect()
    }

    fn update_proxy(&mut self, proxy_index: usize, proxy: ColliderProxy) -> bool {
        let Some(leaf_index) = self.find_leaf(proxy.handle) else {
            let fat_proxy = ColliderProxy {
                handle: proxy.handle,
                aabb: fatten_aabb(proxy.aabb),
            };
            self.insert_leaf(proxy_index, fat_proxy);
            return true;
        };

        self.nodes[leaf_index].proxy_index = Some(proxy_index);
        if aabb_contains(self.nodes[leaf_index].aabb, proxy.aabb) {
            return false;
        }

        self.remove_leaf(leaf_index);
        self.insert_leaf(
            proxy_index,
            ColliderProxy {
                handle: proxy.handle,
                aabb: fatten_aabb(proxy.aabb),
            },
        );
        true
    }

    fn remove_proxy(&mut self, handle: ColliderHandle) -> bool {
        let Some(leaf_index) = self.find_leaf(handle) else {
            return false;
        };
        self.remove_leaf(leaf_index);
        true
    }

    fn find_leaf(&self, handle: ColliderHandle) -> Option<usize> {
        self.leaves.iter().copied().find(|&leaf_index| {
            self.nodes[leaf_index].proxy_index.is_some() && self.nodes[leaf_index].handle == handle
        })
    }

    fn depth(&self) -> usize {
        fn node_depth(nodes: &[TreeNode], index: usize) -> usize {
            let node = &nodes[index];
            if node.is_leaf() {
                return 1;
            }
            let left = node.left.map(|child| node_depth(nodes, child)).unwrap_or(0);
            let right = node
                .right
                .map(|child| node_depth(nodes, child))
                .unwrap_or(0);
            1 + left.max(right)
        }

        self.root
            .map(|root| node_depth(&self.nodes, root))
            .unwrap_or_default()
    }

    fn needs_rebuild(&self) -> bool {
        let leaf_count = self.leaves.len();
        leaf_count >= MIN_REBUILD_LEAF_COUNT && self.depth() > balanced_depth_budget(leaf_count)
    }

    fn needs_compaction(&self) -> bool {
        let leaf_count = self.leaves.len();
        let compact_node_budget = leaf_count.saturating_mul(4).saturating_add(1);
        self.nodes.len() > compact_node_budget
    }

    fn rebuild_balanced_from_proxies(&mut self, proxies: &[ColliderProxy]) {
        let mut entries = proxies
            .iter()
            .copied()
            .enumerate()
            .map(|(proxy_index, proxy)| {
                (
                    proxy_index,
                    ColliderProxy {
                        handle: proxy.handle,
                        aabb: fatten_aabb(proxy.aabb),
                    },
                )
            })
            .collect::<Vec<_>>();

        self.nodes.clear();
        self.root = None;
        self.leaves.clear();
        self.root = self.build_balanced_subtree(&mut entries);
    }

    fn build_balanced_subtree(&mut self, entries: &mut [(usize, ColliderProxy)]) -> Option<usize> {
        match entries.len() {
            0 => None,
            1 => {
                let (proxy_index, proxy) = entries[0];
                let leaf_index = self.nodes.len();
                self.nodes.push(TreeNode {
                    aabb: proxy.aabb,
                    parent: None,
                    left: None,
                    right: None,
                    proxy_index: Some(proxy_index),
                    handle: proxy.handle,
                });
                self.leaves.push(leaf_index);
                Some(leaf_index)
            }
            _ => {
                sort_entries_for_balanced_split(entries);
                let split = entries.len() / 2;
                let (left_entries, right_entries) = entries.split_at_mut(split);
                let left = self
                    .build_balanced_subtree(left_entries)
                    .expect("left balanced broadphase subtree must be non-empty");
                let right = self
                    .build_balanced_subtree(right_entries)
                    .expect("right balanced broadphase subtree must be non-empty");
                let parent_index = self.nodes.len();
                self.nodes.push(TreeNode {
                    aabb: aabb_union(self.nodes[left].aabb, self.nodes[right].aabb),
                    parent: None,
                    left: Some(left),
                    right: Some(right),
                    proxy_index: None,
                    handle: self.nodes[left].handle.min(self.nodes[right].handle),
                });
                self.nodes[left].parent = Some(parent_index);
                self.nodes[right].parent = Some(parent_index);
                Some(parent_index)
            }
        }
    }

    fn insert_leaf(&mut self, proxy_index: usize, proxy: ColliderProxy) {
        let leaf_index = self.nodes.len();
        self.nodes.push(TreeNode {
            aabb: proxy.aabb,
            parent: None,
            left: None,
            right: None,
            proxy_index: Some(proxy_index),
            handle: proxy.handle,
        });
        self.leaves.push(leaf_index);

        let Some(root_index) = self.root else {
            self.root = Some(leaf_index);
            return;
        };

        let sibling = self.choose_sibling(proxy.aabb, root_index);
        let old_parent = self.nodes[sibling].parent;
        let parent_index = self.nodes.len();
        self.nodes.push(TreeNode {
            aabb: aabb_union(proxy.aabb, self.nodes[sibling].aabb),
            parent: old_parent,
            left: Some(sibling),
            right: Some(leaf_index),
            proxy_index: None,
            // Internal nodes carry a deterministic subtree minimum handle for cheap ordering.
            handle: proxy.handle.min(self.nodes[sibling].handle),
        });
        self.nodes[sibling].parent = Some(parent_index);
        self.nodes[leaf_index].parent = Some(parent_index);

        if let Some(old_parent) = old_parent {
            if self.nodes[old_parent].left == Some(sibling) {
                self.nodes[old_parent].left = Some(parent_index);
            } else {
                self.nodes[old_parent].right = Some(parent_index);
            }
        } else {
            self.root = Some(parent_index);
        }

        self.refit_ancestors(Some(parent_index));
    }

    fn remove_leaf(&mut self, leaf_index: usize) {
        if self.root == Some(leaf_index) {
            self.root = None;
            self.leaves.retain(|&index| index != leaf_index);
            self.nodes[leaf_index].parent = None;
            self.nodes[leaf_index].proxy_index = None;
            return;
        }

        let Some(parent_index) = self.nodes[leaf_index].parent else {
            return;
        };
        let sibling_index = if self.nodes[parent_index].left == Some(leaf_index) {
            self.nodes[parent_index]
                .right
                .expect("removed broadphase leaf must have a sibling")
        } else {
            self.nodes[parent_index]
                .left
                .expect("removed broadphase leaf must have a sibling")
        };
        let grandparent = self.nodes[parent_index].parent;

        if let Some(grandparent_index) = grandparent {
            if self.nodes[grandparent_index].left == Some(parent_index) {
                self.nodes[grandparent_index].left = Some(sibling_index);
            } else {
                self.nodes[grandparent_index].right = Some(sibling_index);
            }
            self.nodes[sibling_index].parent = Some(grandparent_index);
            self.refit_ancestors(Some(grandparent_index));
        } else {
            self.root = Some(sibling_index);
            self.nodes[sibling_index].parent = None;
        }

        self.leaves.retain(|&index| index != leaf_index);
        self.nodes[leaf_index].parent = None;
        self.nodes[leaf_index].proxy_index = None;
        self.nodes[parent_index].parent = None;
        self.nodes[parent_index].left = None;
        self.nodes[parent_index].right = None;
    }

    fn choose_sibling(&self, leaf_aabb: ShapeAabb, mut current: usize) -> usize {
        while !self.nodes[current].is_leaf() {
            let left = self.nodes[current]
                .left
                .expect("internal broadphase node must have a left child");
            let right = self.nodes[current]
                .right
                .expect("internal broadphase node must have a right child");

            let left_cost = union_perimeter(leaf_aabb, self.nodes[left].aabb);
            let right_cost = union_perimeter(leaf_aabb, self.nodes[right].aabb);
            current = if left_cost < right_cost {
                left
            } else if right_cost < left_cost {
                right
            } else if self.nodes[left].handle <= self.nodes[right].handle {
                left
            } else {
                right
            };
        }
        current
    }

    fn refit_ancestors(&mut self, mut current: Option<usize>) {
        while let Some(index) = current {
            if let (Some(left), Some(right)) = (self.nodes[index].left, self.nodes[index].right) {
                self.nodes[index].aabb = aabb_union(self.nodes[left].aabb, self.nodes[right].aabb);
                self.nodes[index].handle = self.nodes[left].handle.min(self.nodes[right].handle);
            }
            current = self.nodes[index].parent;
        }
    }

    fn query_overlaps(
        &self,
        query_leaf: usize,
        query_aabb: ShapeAabb,
        mut visit_leaf: impl FnMut(usize),
    ) {
        let Some(root) = self.root else {
            return;
        };
        let mut stack = vec![root];

        while let Some(index) = stack.pop() {
            if index == query_leaf || !aabb_overlaps(query_aabb, self.nodes[index].aabb) {
                continue;
            }
            if self.nodes[index].is_leaf() {
                visit_leaf(index);
                continue;
            }
            if let Some(left) = self.nodes[index].left {
                stack.push(left);
            }
            if let Some(right) = self.nodes[index].right {
                stack.push(right);
            }
        }
    }
}

impl TreeNode {
    fn is_leaf(&self) -> bool {
        self.proxy_index.is_some()
    }
}

fn aabb_union(a: ShapeAabb, b: ShapeAabb) -> ShapeAabb {
    ShapeAabb {
        min: (a.min.x().min(b.min.x()), a.min.y().min(b.min.y())).into(),
        max: (a.max.x().max(b.max.x()), a.max.y().max(b.max.y())).into(),
    }
}

fn fatten_aabb(aabb: ShapeAabb) -> ShapeAabb {
    let width = (aabb.max.x() - aabb.min.x()).abs();
    let height = (aabb.max.y() - aabb.min.y()).abs();
    // A fat AABB is a broadphase cache box. As long as the true shape AABB
    // remains inside it, the tree can skip remove/reinsert for small motion.
    let margin_x = (width * FAT_AABB_EXTENT_RATIO).max(FAT_AABB_MIN_MARGIN);
    let margin_y = (height * FAT_AABB_EXTENT_RATIO).max(FAT_AABB_MIN_MARGIN);
    ShapeAabb {
        min: (aabb.min.x() - margin_x, aabb.min.y() - margin_y).into(),
        max: (aabb.max.x() + margin_x, aabb.max.y() + margin_y).into(),
    }
}

fn balanced_depth_budget(leaf_count: usize) -> usize {
    let balanced_leaf_depth =
        usize::BITS as usize - leaf_count.saturating_sub(1).leading_zeros() as usize;
    balanced_leaf_depth + 2
}

fn sort_entries_for_balanced_split(entries: &mut [(usize, ColliderProxy)]) {
    let (min_x, max_x, min_y, max_y) = entries.iter().fold(
        (
            FloatNum::INFINITY,
            FloatNum::NEG_INFINITY,
            FloatNum::INFINITY,
            FloatNum::NEG_INFINITY,
        ),
        |(min_x, max_x, min_y, max_y), (_, proxy)| {
            let center_x = (proxy.aabb.min.x() + proxy.aabb.max.x()) * 0.5;
            let center_y = (proxy.aabb.min.y() + proxy.aabb.max.y()) * 0.5;
            (
                min_x.min(center_x),
                max_x.max(center_x),
                min_y.min(center_y),
                max_y.max(center_y),
            )
        },
    );
    let split_on_x = (max_x - min_x) >= (max_y - min_y);
    entries.sort_by(|(left_index, left), (right_index, right)| {
        let left_center = if split_on_x {
            (left.aabb.min.x() + left.aabb.max.x()) * 0.5
        } else {
            (left.aabb.min.y() + left.aabb.max.y()) * 0.5
        };
        let right_center = if split_on_x {
            (right.aabb.min.x() + right.aabb.max.x()) * 0.5
        } else {
            (right.aabb.min.y() + right.aabb.max.y()) * 0.5
        };
        left_center
            .total_cmp(&right_center)
            .then_with(|| left_index.cmp(right_index))
    });
}

fn union_perimeter(a: ShapeAabb, b: ShapeAabb) -> FloatNum {
    let union = aabb_union(a, b);
    let width = (union.max.x() - union.min.x()).max(0.0);
    let height = (union.max.y() - union.min.y()).max(0.0);
    2.0 * (width + height)
}

fn aabb_contains(outer: ShapeAabb, inner: ShapeAabb) -> bool {
    outer.min.x() <= inner.min.x()
        && outer.min.y() <= inner.min.y()
        && outer.max.x() >= inner.max.x()
        && outer.max.y() >= inner.max.y()
}

fn aabb_overlaps(a: ShapeAabb, b: ShapeAabb) -> bool {
    let overlap_x = a.max.x().min(b.max.x()) - a.min.x().max(b.min.x());
    let overlap_y = a.max.y().min(b.max.y()) - a.min.y().max(b.min.y());
    overlap_x > 0.0 && overlap_y > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::point::Point;

    fn handle(index: u32) -> ColliderHandle {
        handle_with_generation(index, 0)
    }

    fn handle_with_generation(index: u32, generation: u32) -> ColliderHandle {
        ColliderHandle::from_raw_parts(index, generation)
    }

    fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ShapeAabb {
        ShapeAabb {
            min: Point::new(min_x, min_y),
            max: Point::new(max_x, max_y),
        }
    }

    fn proxy(index: u32, aabb: ShapeAabb) -> ColliderProxy {
        ColliderProxy {
            handle: handle(index),
            aabb,
        }
    }

    fn proxy_with_handle(handle: ColliderHandle, aabb: ShapeAabb) -> ColliderProxy {
        ColliderProxy { handle, aabb }
    }

    fn candidate_pairs(proxies: &[ColliderProxy]) -> Vec<(usize, usize)> {
        DynamicAabbTree::from_proxies(proxies).candidate_pairs()
    }

    #[test]
    fn persistent_broadphase_skips_tree_update_for_motion_inside_fat_aabb() {
        let mut broadphase = Broadphase::default();
        let first = broadphase.update(&[
            proxy(0, aabb(0.0, 0.0, 1.0, 1.0)),
            proxy(1, aabb(4.0, 0.0, 5.0, 1.0)),
        ]);
        assert_eq!(first.stats.update_count, 2);
        assert_eq!(first.stats.candidate_count, 0);
        assert!(first.stats.tree_depth >= 1);

        let contained_motion = broadphase.update(&[
            proxy(0, aabb(0.03, 0.0, 1.03, 1.0)),
            proxy(1, aabb(4.0, 0.0, 5.0, 1.0)),
        ]);
        assert_eq!(contained_motion.stats.update_count, 0);
        assert_eq!(contained_motion.stats.candidate_count, 0);

        let outside_fat_aabb = broadphase.update(&[
            proxy(0, aabb(0.5, 0.0, 1.5, 1.0)),
            proxy(1, aabb(4.0, 0.0, 5.0, 1.0)),
        ]);
        assert_eq!(outside_fat_aabb.stats.update_count, 1);
    }

    #[test]
    fn persistent_broadphase_removes_stale_and_recycled_proxy_handles() {
        let mut broadphase = Broadphase::default();
        let old_handle = handle_with_generation(0, 0);
        let recycled_handle = handle_with_generation(0, 1);
        let stable_handle = handle(1);

        let initial = broadphase.update(&[
            proxy_with_handle(old_handle, aabb(0.0, 0.0, 2.0, 2.0)),
            proxy_with_handle(stable_handle, aabb(1.0, 0.0, 3.0, 2.0)),
        ]);
        assert_eq!(initial.candidate_pairs, vec![(0, 1)]);

        let removed =
            broadphase.update(&[proxy_with_handle(stable_handle, aabb(1.0, 0.0, 3.0, 2.0))]);
        assert!(removed.candidate_pairs.is_empty());
        assert_eq!(removed.stats.update_count, 1);
        assert_eq!(removed.stats.stale_proxy_drop_count, 1);

        let reinserted = broadphase.update(&[
            proxy_with_handle(recycled_handle, aabb(10.0, 0.0, 11.0, 1.0)),
            proxy_with_handle(stable_handle, aabb(1.0, 0.0, 3.0, 2.0)),
        ]);
        assert!(reinserted.candidate_pairs.is_empty());
        assert_eq!(reinserted.stats.update_count, 1);
    }

    #[test]
    fn persistent_broadphase_rebuilds_when_tree_depth_exceeds_budget() {
        let mut broadphase = Broadphase::default();
        let proxies = (0..32)
            .map(|index| {
                let x = index as f32 * 2.0;
                proxy(index, aabb(x, 0.0, x + 1.0, 1.0))
            })
            .collect::<Vec<_>>();

        let output = broadphase.update(&proxies);

        assert_eq!(output.stats.candidate_count, 0);
        assert_eq!(output.stats.rebuild_count, 1);
        assert!(
            output.stats.tree_depth <= balanced_depth_budget(proxies.len()),
            "rebuild should cap tree depth; got {}",
            output.stats.tree_depth
        );
    }

    #[test]
    fn persistent_broadphase_compacts_small_scene_tombstones_after_churn() {
        let mut broadphase = Broadphase::default();
        let stable = proxy(1, aabb(10.0, 0.0, 11.0, 1.0));
        let mut saw_compaction = false;

        for step in 0..24 {
            let x = step as f32;
            let output = broadphase.update(&[proxy(0, aabb(x, 0.0, x + 1.0, 1.0)), stable]);
            saw_compaction |= output.stats.rebuild_count > 0;
        }

        assert!(
            saw_compaction,
            "small scenes should eventually compact tombstones"
        );
        assert!(
            broadphase.tree.nodes.len() <= broadphase.tree.leaves.len() * 4 + 1,
            "compaction should bound retained nodes; nodes={}, leaves={}",
            broadphase.tree.nodes.len(),
            broadphase.tree.leaves.len()
        );
    }

    #[test]
    fn empty_and_single_proxy_tree_yield_no_candidate_pairs() {
        assert!(candidate_pairs(&[]).is_empty());
        assert!(candidate_pairs(&[proxy(0, aabb(0.0, 0.0, 1.0, 1.0))]).is_empty());
    }

    #[test]
    fn sparse_aabbs_yield_only_overlapping_candidate_pairs() {
        let proxies = vec![
            proxy(10, aabb(0.0, 0.0, 2.0, 2.0)),
            proxy(30, aabb(10.0, 10.0, 11.0, 11.0)),
            proxy(20, aabb(1.0, 1.0, 3.0, 3.0)),
            proxy(40, aabb(2.0, 4.0, 3.0, 5.0)),
        ];

        assert_eq!(candidate_pairs(&proxies), vec![(0, 2)]);
    }

    #[test]
    fn dense_aabbs_yield_all_expected_pairs() {
        let proxies = vec![
            proxy(1, aabb(0.0, 0.0, 4.0, 4.0)),
            proxy(2, aabb(1.0, 1.0, 5.0, 5.0)),
            proxy(3, aabb(2.0, 2.0, 6.0, 6.0)),
            proxy(4, aabb(3.0, 3.0, 7.0, 7.0)),
        ];

        assert_eq!(
            candidate_pairs(&proxies),
            vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
        );
    }

    #[test]
    fn candidate_pairs_are_ordered_by_snapshot_index_without_self_or_reverse_duplicates() {
        let proxies = vec![
            proxy(30, aabb(0.0, 0.0, 3.0, 3.0)),
            proxy(10, aabb(1.0, 1.0, 4.0, 4.0)),
            proxy(20, aabb(2.0, 2.0, 5.0, 5.0)),
        ];

        let pairs = candidate_pairs(&proxies);

        assert_eq!(pairs, vec![(0, 1), (0, 2), (1, 2)]);
        assert!(pairs.iter().all(|(a, b)| a != b));
        for (a, b) in &pairs {
            assert!(
                !pairs.contains(&(*b, *a)),
                "candidate pair {a:?}/{b:?} was duplicated in reverse"
            );
        }
    }

    #[test]
    fn recycled_handle_order_does_not_change_candidate_index_order() {
        let proxies = vec![
            proxy_with_handle(handle_with_generation(0, 1), aabb(0.0, 0.0, 2.0, 2.0)),
            proxy_with_handle(handle(1), aabb(1.0, 1.0, 3.0, 3.0)),
        ];

        assert_eq!(candidate_pairs(&proxies), vec![(0, 1)]);
    }
}
