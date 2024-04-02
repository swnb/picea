use crate::{
    algo::sort::SortableCollection,
    collision::{
        accurate_collision_detection_for_sub_collider, prepare_accurate_collision_detection,
        rough_collision_detection, CollisionalCollection, ContactPointPair,
    },
};

use super::{Element, ID};
use std::{
    cell::UnsafeCell,
    cmp::Ordering,
    collections::BTreeMap,
    ops::{Index, IndexMut},
    rc::Rc,
};

struct StoredElement<D: Clone> {
    element: UnsafeCell<Element<D>>,
}

impl<D: Clone> StoredElement<D> {
    fn element(&self) -> &Element<D> {
        unsafe { &*self.element.get() }
    }

    fn element_mut(&self) -> *mut Element<D> {
        self.element.get()
    }
}

/**
 * ElementStore store all element with sort result cache
 */
#[derive(Default, Clone)]
pub struct ElementStore<T: Clone> {
    elements: Vec<Rc<StoredElement<T>>>,     // origin element order
    region_sort_result: Vec<ID>,             // "sweep_and_prune_collision_detection" sort cache
    map: BTreeMap<ID, Rc<StoredElement<T>>>, // indexMap find origin element index by element id
    // TODO remove this field
    is_sorted: bool, // use quick sort algo , otherwise use select sort
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
        self.elements.iter().map(|v| v.element())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Element<T>> {
        self.elements
            .iter_mut()
            .map(|v| unsafe { &mut *v.element_mut() })
    }

    pub fn push(&mut self, element: Element<T>) {
        let id = element.id;
        let element = StoredElement {
            element: UnsafeCell::new(element),
        };
        let element = Rc::new(element);
        self.elements.push(element.clone());
        self.region_sort_result.push(id);
        self.map.insert(id, element);
        self.is_sorted = false;
    }

    pub fn has_element(&self, id: ID) -> bool {
        self.map.contains_key(&id)
    }

    pub fn remove_element(&mut self, id: ID) {
        self.elements.retain(|v| v.element().id != id);
        self.map.remove(&id);
        self.is_sorted = false;
    }

    pub fn clear(&mut self) {
        self.elements.clear();
        self.region_sort_result.clear();
        self.map.clear();
        self.is_sorted = false;
    }

    pub fn get_element_by_id(&self, id: ID) -> Option<&Element<T>> {
        self.map.get(&id).map(|v| v.element())
    }

    pub fn get_mut_element_by_id(&mut self, id: ID) -> Option<&mut Element<T>> {
        let value = self.map.get_mut(&id)?;
        (unsafe { &mut *value.element_mut() }).into()
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
                let id = v.element().id;
                self.region_sort_result.push(id);
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
