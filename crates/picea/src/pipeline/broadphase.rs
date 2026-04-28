use std::collections::{BTreeMap, BTreeSet};

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
    pub(crate) traversal_count: usize,
    pub(crate) pruned_count: usize,
    pub(crate) rebuild_count: usize,
    pub(crate) tree_depth: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BroadphaseOutput {
    pub(crate) candidate_pairs: Vec<(usize, usize)>,
    pub(crate) stats: BroadphaseStats,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TreeQueryStats {
    pub(crate) traversal_count: usize,
    pub(crate) pruned_count: usize,
    pub(crate) candidate_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TreeQueryOutput {
    pub(crate) proxy_indices: Vec<usize>,
    pub(crate) stats: TreeQueryStats,
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

        let candidate_output = self.tree.candidate_pairs_with_stats();
        BroadphaseOutput {
            stats: BroadphaseStats {
                candidate_count: candidate_output.pairs.len(),
                update_count,
                stale_proxy_drop_count,
                same_body_drop_count: 0,
                filter_drop_count: 0,
                narrowphase_drop_count: 0,
                traversal_count: candidate_output.stats.traversal_count,
                pruned_count: candidate_output.stats.pruned_count,
                rebuild_count,
                tree_depth: self.tree.depth(),
            },
            candidate_pairs: candidate_output.pairs,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CandidatePairOutput {
    pairs: Vec<(usize, usize)>,
    stats: TreeQueryStats,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DynamicAabbTree {
    nodes: Vec<TreeNode>,
    root: Option<usize>,
    leaves: Vec<usize>,
    leaf_by_handle: BTreeMap<ColliderHandle, usize>,
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
    pub(crate) fn from_proxies(proxies: &[ColliderProxy]) -> Self {
        let mut tree = Self::default();
        let mut entries = proxies.iter().copied().enumerate().collect::<Vec<_>>();
        // Query snapshots are built in one shot. Use the same deterministic
        // balanced builder as rebuilds, but keep exact AABBs instead of fattening
        // them so query pruning cannot broaden public hit semantics.
        tree.root = tree.build_balanced_subtree(&mut entries);
        tree
    }

    #[cfg(test)]
    pub(crate) fn query_aabb_proxy_indices(&self, query_aabb: ShapeAabb) -> Vec<usize> {
        self.query_aabb_proxy_indices_with_stats(query_aabb)
            .proxy_indices
    }

    pub(crate) fn query_aabb_proxy_indices_with_stats(
        &self,
        query_aabb: ShapeAabb,
    ) -> TreeQueryOutput {
        let mut indices = Vec::new();
        let stats = self.query_indices(
            |node_aabb| aabb_overlaps_inclusive(query_aabb, node_aabb),
            &mut indices,
        );
        TreeQueryOutput {
            proxy_indices: indices,
            stats,
        }
    }

    pub(crate) fn query_point_proxy_indices_with_stats(
        &self,
        point: crate::math::point::Point,
    ) -> TreeQueryOutput {
        let mut indices = Vec::new();
        let stats = self.query_indices(
            |node_aabb| aabb_contains_point(node_aabb, point),
            &mut indices,
        );
        TreeQueryOutput {
            proxy_indices: indices,
            stats,
        }
    }

    pub(crate) fn query_ray_proxy_indices_with_stats(
        &self,
        origin: crate::math::point::Point,
        direction: crate::math::vector::Vector,
        max_toi: FloatNum,
    ) -> TreeQueryOutput {
        let mut indices = Vec::new();
        let stats = self.query_indices(
            |node_aabb| ray_intersects_aabb(origin, direction, max_toi, node_aabb),
            &mut indices,
        );
        TreeQueryOutput {
            proxy_indices: indices,
            stats,
        }
    }

    #[cfg(test)]
    pub(crate) fn candidate_pairs(&self) -> Vec<(usize, usize)> {
        self.candidate_pairs_with_stats().pairs
    }

    fn candidate_pairs_with_stats(&self) -> CandidatePairOutput {
        let mut pairs = Vec::new();
        let mut stats = TreeQueryStats::default();

        if let Some(root) = self.root {
            self.collect_subtree_candidate_pairs(root, &mut pairs, &mut stats);
        }

        // Preserve the old contact pass ordering: live collider snapshot order,
        // not handle ordering. Recycled handles may compare differently because
        // generation bits participate in `Ord`.
        pairs.sort_unstable();
        stats.candidate_count = pairs.len();
        CandidatePairOutput { pairs, stats }
    }

    fn collect_subtree_candidate_pairs(
        &self,
        node_index: usize,
        pairs: &mut Vec<(usize, usize)>,
        stats: &mut TreeQueryStats,
    ) {
        let node = &self.nodes[node_index];
        let (Some(left), Some(right)) = (node.left, node.right) else {
            return;
        };

        self.collect_subtree_candidate_pairs(left, pairs, stats);
        self.collect_subtree_candidate_pairs(right, pairs, stats);
        self.collect_candidate_pairs_between(left, right, pairs, stats);
    }

    fn collect_candidate_pairs_between(
        &self,
        left_index: usize,
        right_index: usize,
        pairs: &mut Vec<(usize, usize)>,
        stats: &mut TreeQueryStats,
    ) {
        stats.traversal_count += 1;

        let left = &self.nodes[left_index];
        let right = &self.nodes[right_index];
        if !aabb_overlaps(left.aabb, right.aabb) {
            stats.pruned_count += 1;
            return;
        }

        match (left.proxy_index, right.proxy_index) {
            (Some(left_proxy_index), Some(right_proxy_index)) => {
                let (first, second) = if left_proxy_index <= right_proxy_index {
                    (left_proxy_index, right_proxy_index)
                } else {
                    (right_proxy_index, left_proxy_index)
                };
                if first != second {
                    stats.candidate_count += 1;
                    pairs.push((first, second));
                }
            }
            (Some(_), None) => {
                let right_left = right
                    .left
                    .expect("internal broadphase node must have a left child");
                let right_right = right
                    .right
                    .expect("internal broadphase node must have a right child");
                self.collect_candidate_pairs_between(left_index, right_left, pairs, stats);
                self.collect_candidate_pairs_between(left_index, right_right, pairs, stats);
            }
            (None, Some(_)) => {
                let left_left = left
                    .left
                    .expect("internal broadphase node must have a left child");
                let left_right = left
                    .right
                    .expect("internal broadphase node must have a right child");
                self.collect_candidate_pairs_between(left_left, right_index, pairs, stats);
                self.collect_candidate_pairs_between(left_right, right_index, pairs, stats);
            }
            (None, None) => {
                let left_left = left
                    .left
                    .expect("internal broadphase node must have a left child");
                let left_right = left
                    .right
                    .expect("internal broadphase node must have a right child");
                let right_left = right
                    .left
                    .expect("internal broadphase node must have a left child");
                let right_right = right
                    .right
                    .expect("internal broadphase node must have a right child");

                // Walk each overlapping subtree pair once from its lowest common
                // ancestor so traversal/prune counters measure subtree-pair work
                // units rather than literal node visits.
                self.collect_candidate_pairs_between(left_left, right_left, pairs, stats);
                self.collect_candidate_pairs_between(left_left, right_right, pairs, stats);
                self.collect_candidate_pairs_between(left_right, right_left, pairs, stats);
                self.collect_candidate_pairs_between(left_right, right_right, pairs, stats);
            }
        }
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
        self.leaf_index_for(handle)
    }

    fn leaf_index_for(&self, handle: ColliderHandle) -> Option<usize> {
        self.leaf_by_handle
            .get(&handle)
            .copied()
            .filter(|&leaf_index| {
                self.nodes
                    .get(leaf_index)
                    .is_some_and(|node| node.proxy_index.is_some() && node.handle == handle)
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
        self.leaf_by_handle.clear();
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
                self.leaf_by_handle.insert(proxy.handle, leaf_index);
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
        self.leaf_by_handle.insert(proxy.handle, leaf_index);

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
        if let Some(handle) = self.nodes.get(leaf_index).map(|node| node.handle) {
            self.leaf_by_handle.remove(&handle);
        }

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

    fn query_indices(
        &self,
        mut overlaps: impl FnMut(ShapeAabb) -> bool,
        out: &mut Vec<usize>,
    ) -> TreeQueryStats {
        let Some(root) = self.root else {
            return TreeQueryStats::default();
        };
        let mut stack = vec![root];
        let mut stats = TreeQueryStats::default();

        while let Some(index) = stack.pop() {
            stats.traversal_count += 1;
            let node = &self.nodes[index];
            if !overlaps(node.aabb) {
                stats.pruned_count += 1;
                continue;
            }
            if let Some(proxy_index) = node.proxy_index {
                stats.candidate_count += 1;
                out.push(proxy_index);
                continue;
            }
            if let Some(right) = node.right {
                stack.push(right);
            }
            if let Some(left) = node.left {
                stack.push(left);
            }
        }

        // Query callers keep the stable public ordering contract: snapshot order.
        out.sort_unstable();
        stats
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

fn aabb_overlaps_inclusive(a: ShapeAabb, b: ShapeAabb) -> bool {
    !(a.max.x() < b.min.x()
        || b.max.x() < a.min.x()
        || a.max.y() < b.min.y()
        || b.max.y() < a.min.y())
}

fn aabb_contains_point(aabb: ShapeAabb, point: crate::math::point::Point) -> bool {
    point.x() >= aabb.min.x()
        && point.x() <= aabb.max.x()
        && point.y() >= aabb.min.y()
        && point.y() <= aabb.max.y()
}

fn ray_intersects_aabb(
    origin: crate::math::point::Point,
    direction: crate::math::vector::Vector,
    max_toi: FloatNum,
    aabb: ShapeAabb,
) -> bool {
    let mut enter: FloatNum = 0.0;
    let mut exit = max_toi.max(0.0);

    for (origin_axis, direction_axis, min_axis, max_axis) in [
        (origin.x(), direction.x(), aabb.min.x(), aabb.max.x()),
        (origin.y(), direction.y(), aabb.min.y(), aabb.max.y()),
    ] {
        if direction_axis.abs() <= FloatNum::EPSILON {
            if origin_axis < min_axis || origin_axis > max_axis {
                return false;
            }
            continue;
        }
        let inverse = 1.0 / direction_axis;
        let mut t0 = (min_axis - origin_axis) * inverse;
        let mut t1 = (max_axis - origin_axis) * inverse;
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        enter = enter.max(t0);
        exit = exit.min(t1);
        if enter > exit {
            return false;
        }
    }

    exit >= 0.0 && enter <= max_toi
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

    #[test]
    fn candidate_pair_traversal_visits_each_overlapping_subtree_once() {
        let tree = DynamicAabbTree::from_proxies(&[
            proxy(10, aabb(0.0, 0.0, 2.0, 2.0)),
            proxy(20, aabb(1.5, 0.0, 3.5, 2.0)),
            proxy(30, aabb(20.0, 20.0, 22.0, 22.0)),
        ]);

        let output = tree.candidate_pairs_with_stats();

        assert_eq!(output.pairs, vec![(0, 1)]);
        assert_eq!(output.stats.candidate_count, 1);
        assert_eq!(output.stats.traversal_count, 4);
        assert_eq!(output.stats.pruned_count, 2);
    }

    #[test]
    fn query_traversal_returns_proxy_indices_in_snapshot_order() {
        let tree = DynamicAabbTree::from_proxies(&[
            proxy(30, aabb(6.0, 0.0, 8.0, 2.0)),
            proxy(10, aabb(0.0, 0.0, 2.0, 2.0)),
            proxy(20, aabb(1.0, 1.0, 3.0, 3.0)),
            proxy(40, aabb(10.0, 0.0, 12.0, 2.0)),
        ]);

        assert_eq!(
            tree.query_aabb_proxy_indices(aabb(-1.0, -1.0, 4.0, 4.0)),
            vec![1, 2]
        );
        assert_eq!(
            tree.query_aabb_proxy_indices(aabb(-1.0, -1.0, 9.0, 4.0)),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn query_tree_builds_balanced_from_ordered_snapshot_proxies() {
        let proxies = (0..64)
            .map(|index| {
                let x = index as f32 * 2.0;
                proxy(index, aabb(x, 0.0, x + 1.0, 1.0))
            })
            .collect::<Vec<_>>();

        let tree = DynamicAabbTree::from_proxies(&proxies);

        assert!(
            tree.depth() <= balanced_depth_budget(proxies.len()),
            "query tree should not inherit insertion-order depth; got {}",
            tree.depth()
        );
    }

    #[test]
    fn query_traversal_tracks_stale_removal_rebuilds_and_recycled_handles() {
        let mut broadphase = Broadphase::default();
        let old_handle = handle_with_generation(0, 0);
        let recycled_handle = handle_with_generation(0, 1);
        let stable_handle = handle(1);

        broadphase.update(&[
            proxy_with_handle(old_handle, aabb(0.0, 0.0, 2.0, 2.0)),
            proxy_with_handle(stable_handle, aabb(5.0, 0.0, 7.0, 2.0)),
        ]);
        assert_eq!(
            broadphase
                .tree
                .query_aabb_proxy_indices(aabb(-1.0, -1.0, 3.0, 3.0)),
            vec![0]
        );

        broadphase.update(&[proxy_with_handle(stable_handle, aabb(5.0, 0.0, 7.0, 2.0))]);
        assert!(broadphase
            .tree
            .query_aabb_proxy_indices(aabb(-1.0, -1.0, 3.0, 3.0))
            .is_empty());

        let many_proxies = (100..132)
            .map(|index| {
                let x = index as f32 * 2.0;
                proxy(index, aabb(x, 2.0, x + 1.0, 3.0))
            })
            .collect::<Vec<_>>();
        broadphase.update(&many_proxies);
        assert!(broadphase
            .tree
            .query_aabb_proxy_indices(aabb(-1.0, -1.0, 8.0, 8.0))
            .is_empty());

        broadphase.update(&[
            proxy_with_handle(recycled_handle, aabb(0.0, 8.0, 1.0, 9.0)),
            proxy_with_handle(stable_handle, aabb(0.5, 8.0, 1.5, 9.5)),
        ]);
        assert_eq!(
            broadphase
                .tree
                .query_aabb_proxy_indices(aabb(-1.0, 7.0, 2.0, 10.0)),
            vec![0, 1]
        );
    }

    #[test]
    fn leaf_lookup_tracks_moves_rebuilds_stale_removal_and_recycled_handles() {
        let mut broadphase = Broadphase::default();
        let old_handle = handle_with_generation(0, 0);
        let recycled_handle = handle_with_generation(0, 1);
        let stable_handle = handle(1);

        broadphase.update(&[
            proxy_with_handle(old_handle, aabb(0.0, 0.0, 1.0, 1.0)),
            proxy_with_handle(stable_handle, aabb(3.0, 0.0, 4.0, 1.0)),
        ]);

        let stable_leaf = broadphase
            .tree
            .leaf_index_for(stable_handle)
            .expect("stable collider should have a leaf");
        assert_eq!(broadphase.tree.nodes[stable_leaf].handle, stable_handle);

        broadphase.update(&[
            proxy_with_handle(old_handle, aabb(6.0, 0.0, 7.0, 1.0)),
            proxy_with_handle(stable_handle, aabb(3.0, 0.0, 4.0, 1.0)),
        ]);
        let moved_leaf = broadphase
            .tree
            .leaf_index_for(old_handle)
            .expect("moved collider should still be directly indexed");
        assert_eq!(broadphase.tree.nodes[moved_leaf].handle, old_handle);

        let many_proxies = (100..132)
            .map(|index| {
                let x = index as f32 * 2.0;
                proxy(index, aabb(x, 2.0, x + 1.0, 3.0))
            })
            .collect::<Vec<_>>();
        broadphase.update(&many_proxies);
        assert!(broadphase.tree.leaf_index_for(old_handle).is_none());
        assert!(broadphase.tree.leaf_index_for(stable_handle).is_none());
        for proxy in &many_proxies {
            let leaf = broadphase
                .tree
                .leaf_index_for(proxy.handle)
                .expect("rebuilt tree should keep every live handle indexed");
            assert_eq!(broadphase.tree.nodes[leaf].handle, proxy.handle);
        }

        broadphase.update(&[proxy_with_handle(recycled_handle, aabb(0.0, 8.0, 1.0, 9.0))]);

        assert!(broadphase.tree.leaf_index_for(old_handle).is_none());
        let recycled_leaf = broadphase
            .tree
            .leaf_index_for(recycled_handle)
            .expect("recycled generation should be indexed independently");
        assert_eq!(broadphase.tree.nodes[recycled_leaf].handle, recycled_handle);
    }
}
