use crate::{
    algo::sort::SortableCollection,
    collision::{
        accurate_collision_detection_for_sub_collider, prepare_accurate_collision_detection,
        rough_collision_detection, CollisionalCollection, ContactPointPair,
    },
};

use super::{Element, ID};
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    ops::{Index, IndexMut},
};

/**
 * ElementStore store all element with sort result cache
 */
#[derive(Default)]
pub struct ElementStore<T: Clone> {
    elements: Vec<Box<Element<T>>>, // origin element order
    region_sort_result: Vec<ID>,    // "sweep_and_prune_collision_detection" sort cache
    map: BTreeMap<ID, usize>,       // find origin element index by element id
    // TODO remove this field
    is_sorted: bool, // use quick sort algo , otherwise use select sort
}

impl<T: Clone> Clone for ElementStore<T> {
    fn clone(&self) -> Self {
        // Store clone preserves ids and bind points; Element::clone() intentionally resets them.
        let elements = self
            .elements
            .iter()
            .map(|element| Box::new(clone_element_preserving_store_state(element)))
            .collect();
        let mut cloned = Self {
            elements,
            region_sort_result: self.region_sort_result.clone(),
            map: BTreeMap::new(),
            is_sorted: self.is_sorted,
        };
        cloned.rebuild_map();
        cloned
    }
}

impl<T: Clone> Index<usize> for ElementStore<T> {
    type Output = Element<T>;
    fn index(&self, index: usize) -> &Self::Output {
        let id = self.region_sort_result[index];
        self.get_element_by_id(id).unwrap()
    }
}

impl<T: Clone> SortableCollection for ElementStore<T> {
    type Item = Element<T>;

    fn len(&self) -> usize {
        self.elements.len()
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.region_sort_result.swap(i, j)
    }
}

impl<T: Clone> ElementStore<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
            region_sort_result: Vec::with_capacity(capacity),
            map: BTreeMap::new(),
            is_sorted: false,
        }
    }

    pub fn size(&self) -> usize {
        // TODO remove delete element when count
        self.elements.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Element<T>> {
        self.elements.iter().map(Box::as_ref)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Element<T>> {
        self.elements.iter_mut().map(Box::as_mut)
    }

    pub fn push(&mut self, element: Element<T>) {
        let id = element.id;

        if self.has_element(id) {
            self.remove_element(id);
        }

        let index = self.elements.len();
        self.elements.push(Box::new(element));
        self.region_sort_result.push(id);
        self.map.insert(id, index);
        self.is_sorted = false;
    }

    pub fn has_element(&self, id: ID) -> bool {
        self.map.contains_key(&id)
    }

    pub fn remove_element(&mut self, id: ID) {
        let Some(index) = self.map.remove(&id) else {
            return;
        };

        self.elements.remove(index);
        self.region_sort_result.retain(|cached_id| *cached_id != id);
        self.reindex_from(index);
        self.is_sorted = false;
    }

    pub fn clear(&mut self) {
        self.elements.clear();
        self.region_sort_result.clear();
        self.map.clear();
        self.is_sorted = false;
    }

    pub fn get_element_by_id(&self, id: ID) -> Option<&Element<T>> {
        let index = *self.map.get(&id)?;
        self.elements.get(index).map(Box::as_ref)
    }

    pub fn get_mut_element_by_id(&mut self, id: ID) -> Option<&mut Element<T>> {
        let index = *self.map.get(&id)?;
        self.elements.get_mut(index).map(Box::as_mut)
    }

    pub(crate) fn get_pair_mut_by_id(
        &mut self,
        element_a_id: ID,
        element_b_id: ID,
    ) -> Option<(&mut Element<T>, &mut Element<T>)> {
        if element_a_id == element_b_id {
            return None;
        }

        let index_a = *self.map.get(&element_a_id)?;
        let index_b = *self.map.get(&element_b_id)?;

        match index_a.cmp(&index_b) {
            Ordering::Less => {
                let (left, right) = self.elements.split_at_mut(index_b);
                Some((left[index_a].as_mut(), right[0].as_mut()))
            }
            Ordering::Greater => {
                let (left, right) = self.elements.split_at_mut(index_a);
                Some((right[0].as_mut(), left[index_b].as_mut()))
            }
            Ordering::Equal => None,
        }
    }

    pub fn sort<F>(&mut self, compare: F)
    where
        F: Fn(&Element<T>, &Element<T>) -> Ordering,
    {
        if self.is_sorted {
            self.insertion_sort(compare);
        } else {
            // TODO opt
            self.region_sort_result.truncate(0);
            for v in &self.elements {
                self.region_sort_result.push(v.id);
            }
            self.quick_sort(compare);
            // turn off quick sort
            self.is_sorted = true;
        }
    }

    pub fn detective_collision(
        &mut self,
        mut handler: impl FnMut(&Element<T>, &Element<T>, Vec<ContactPointPair>),
    ) {
        rough_collision_detection(self, |element_a, element_b| {
            let should_skip = {
                let meta_a = element_a.meta();
                let meta_b = element_b.meta();

                let is_both_sleeping = meta_a.is_sleeping() && meta_b.is_sleeping();

                is_both_sleeping || meta_a.is_transparent() || meta_b.is_transparent()
            };

            if should_skip {
                return;
            }

            let (collider_a, collider_b) = if element_a.id() > element_b.id() {
                (element_b, element_a)
            } else {
                (element_a, element_b)
            };

            prepare_accurate_collision_detection(
                collider_a,
                collider_b,
                |sub_collider_a, sub_collider_b| {
                    if let Some(contact_pairs) = accurate_collision_detection_for_sub_collider(
                        sub_collider_a,
                        sub_collider_b,
                    ) {
                        let a = collider_a;
                        let b = collider_b;
                        handler(a, b, contact_pairs);
                    }
                },
            )
        });
    }

    fn rebuild_map(&mut self) {
        self.map.clear();
        self.reindex_from(0);
    }

    fn reindex_from(&mut self, start_index: usize) {
        for (offset, element) in self.elements[start_index..].iter().enumerate() {
            self.map.insert(element.id, start_index + offset);
        }
    }
}

fn clone_element_preserving_store_state<T: Clone>(element: &Element<T>) -> Element<T> {
    Element {
        id: element.id,
        meta: element.meta.clone(),
        shape: element.shape.self_clone(),
        bind_points: element.bind_points.clone(),
        data: element.data.clone(),
    }
}

impl<T: Clone> Index<usize> for &mut ElementStore<T> {
    type Output = Element<T>;

    fn index(&self, index: usize) -> &Self::Output {
        let id = self.region_sort_result[index];
        self.get_element_by_id(id).unwrap()
    }
}

impl<T: Clone> IndexMut<usize> for &mut ElementStore<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let id = self.region_sort_result[index];
        self.get_mut_element_by_id(id).unwrap()
    }
}

impl<T: Clone> CollisionalCollection for &mut ElementStore<T> {
    type Collider = Element<T>;
    fn len(&self) -> usize {
        self.region_sort_result.len()
    }

    fn sort(&mut self, compare: impl Fn(&Element<T>, &Element<T>) -> Ordering) {
        ElementStore::sort(self, compare)
    }
}

#[cfg(test)]
mod tests {
    use super::ElementStore;
    use crate::{
        element::{Element, ElementBuilder, ID},
        math::vector::Vector,
        meta::MetaBuilder,
        shape::Circle,
    };

    fn circle_element(id: ID, x: f32) -> Element<()> {
        let mut element: Element<()> =
            ElementBuilder::new(Circle::new((x, 0.), 1.), MetaBuilder::new().mass(1.), ()).into();
        element.inject_id(id);
        element
    }

    fn element_ids(store: &ElementStore<()>) -> Vec<ID> {
        store.iter().map(|element| element.id).collect()
    }

    fn set_velocity(element: &mut Element<()>, velocity: impl Into<Vector>) {
        *element.meta_mut().velocity_mut() = velocity.into();
    }

    #[test]
    fn add_get_and_remove_keep_order_map_and_sort_cache_consistent() {
        let mut store = ElementStore::with_capacity(3);
        store.push(circle_element(1, 3.));
        store.push(circle_element(2, 1.));
        store.push(circle_element(3, 2.));

        assert_eq!(element_ids(&store), vec![1, 2, 3]);
        assert_eq!(
            store.get_element_by_id(2).map(|element| element.id),
            Some(2)
        );

        store.sort(|a, b| {
            a.center_point()
                .x()
                .partial_cmp(&b.center_point().x())
                .unwrap()
        });
        assert_eq!(store.region_sort_result, vec![2, 3, 1]);

        store.remove_element(3);

        assert_eq!(store.size(), 2);
        assert_eq!(element_ids(&store), vec![1, 2]);
        assert!(!store.has_element(3));
        assert!(store.get_element_by_id(3).is_none());
        assert_eq!(store.region_sort_result, vec![2, 1]);

        set_velocity(
            store
                .get_mut_element_by_id(2)
                .expect("shifted map index still points at id 2"),
            (7., 0.),
        );
        assert_eq!(
            store
                .get_element_by_id(2)
                .expect("id 2 still exists")
                .meta()
                .velocity()
                .x(),
            7.
        );
    }

    #[test]
    fn removing_non_tail_element_reindexes_shifted_map_entries() {
        let mut store = ElementStore::with_capacity(3);
        store.push(circle_element(1, 10.));
        store.push(circle_element(2, 20.));
        store.push(circle_element(3, 30.));

        store.remove_element(1);

        assert_eq!(store.size(), 2);
        assert_eq!(element_ids(&store), vec![2, 3]);
        assert!(store.get_element_by_id(1).is_none());
        assert_eq!(
            store
                .get_element_by_id(2)
                .expect("id 2 shifts to index 0")
                .center_point()
                .x(),
            20.
        );
        assert_eq!(
            store
                .get_element_by_id(3)
                .expect("id 3 shifts to index 1")
                .center_point()
                .x(),
            30.
        );

        set_velocity(
            store
                .get_mut_element_by_id(3)
                .expect("mutable lookup uses shifted index for id 3"),
            (9., 0.),
        );
        assert_eq!(
            store
                .get_element_by_id(3)
                .expect("id 3 remains accessible")
                .meta()
                .velocity()
                .x(),
            9.
        );

        let (element_2, element_3) = store
            .get_pair_mut_by_id(2, 3)
            .expect("pair lookup uses reindexed map entries");
        set_velocity(element_2, (2., 0.));
        set_velocity(element_3, (3., 0.));

        assert_eq!(
            store
                .get_element_by_id(2)
                .expect("id 2 remains accessible")
                .meta()
                .velocity()
                .x(),
            2.
        );
        assert_eq!(
            store
                .get_element_by_id(3)
                .expect("id 3 remains accessible")
                .meta()
                .velocity()
                .x(),
            3.
        );
    }

    #[test]
    fn duplicate_id_replaces_existing_entry_without_cache_duplicates() {
        let mut store = ElementStore::with_capacity(2);
        store.push(circle_element(1, 0.));
        store.push(circle_element(1, 10.));

        assert_eq!(store.size(), 1);
        assert_eq!(element_ids(&store), vec![1]);
        assert_eq!(store.region_sort_result, vec![1]);
        assert_eq!(
            store
                .get_element_by_id(1)
                .expect("replacement remains accessible")
                .center_point()
                .x(),
            10.
        );
    }

    #[test]
    fn clone_preserves_ids_without_sharing_element_mutation() {
        let mut store = ElementStore::with_capacity(2);
        store.push(circle_element(1, 0.));
        store.push(circle_element(2, 1.));

        let mut cloned = store.clone();
        set_velocity(
            cloned
                .get_mut_element_by_id(1)
                .expect("cloned id remains accessible"),
            (8., 0.),
        );

        assert_eq!(element_ids(&cloned), vec![1, 2]);
        assert_eq!(
            cloned
                .get_element_by_id(1)
                .expect("cloned id 1 exists")
                .meta()
                .velocity()
                .x(),
            8.
        );
        assert_eq!(
            store
                .get_element_by_id(1)
                .expect("original id 1 exists")
                .meta()
                .velocity()
                .x(),
            0.
        );
    }
}
