use crate::{collider::ShapeAabb, handles::ColliderHandle, math::FloatNum};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ColliderProxy {
    pub(crate) handle: ColliderHandle,
    pub(crate) aabb: ShapeAabb,
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

fn union_perimeter(a: ShapeAabb, b: ShapeAabb) -> FloatNum {
    let union = aabb_union(a, b);
    let width = (union.max.x() - union.min.x()).max(0.0);
    let height = (union.max.y() - union.min.y()).max(0.0);
    2.0 * (width + height)
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
