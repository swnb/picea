use crate::algo::{collision::CollisionalCollection, sort::SortableCollection};

use super::{Element, ID};
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    ops::{Index, IndexMut},
    rc::Rc,
};

struct StoredElement<E = Element> {
    is_deleted: bool,
    element: E,
}

/**
 * ElementStore store all element with sort result cache
 */
#[derive(Default)]
pub struct ElementStore {
    elements: Vec<Rc<StoredElement>>,     // origin element order
    region_sort_result: Vec<ID>,          // "sweep_and_prune_collision_detection" sort cache
    map: BTreeMap<ID, Rc<StoredElement>>, // indexMap find origin element index by element id
    // TODO remove this field
    is_sorted: bool, // use quick sort algo , otherwise use select sort
}

impl Index<usize> for ElementStore {
    type Output = Element;
    fn index(&self, index: usize) -> &Self::Output {
        let id = self.region_sort_result[index];
        self.get_element_by_id(id).unwrap()
    }
}

impl SortableCollection for ElementStore {
    type Item = Element;

    fn len(&self) -> usize {
        self.elements.len()
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.region_sort_result.swap(i, j)
    }
}

impl ElementStore {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
            region_sort_result: Vec::with_capacity(capacity),
            map: BTreeMap::new(),
            is_sorted: false,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Element> {
        self.elements.iter().map(|v| &v.element)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Element> {
        self.elements.iter_mut().map(|v| {
            let element = &v.element as *const _ as *mut _;
            unsafe { &mut *element }
        })
    }

    pub fn push(&mut self, element: Element) {
        let id = element.id;
        let element = StoredElement {
            is_deleted: false,
            element,
        };
        let element = Rc::new(element);
        self.elements.push(element.clone());
        self.map.insert(id, element);
        self.is_sorted = false;
    }

    pub fn remove(&mut self, id: ID) {
        // TODO
    }

    pub fn get_element_by_id(&self, id: ID) -> Option<&Element> {
        self.map.get(&id).map(|v| &v.element)
    }

    pub fn get_mut_element_by_id(&mut self, id: ID) -> Option<&mut Element> {
        self.map.get_mut(&id).map(|v| {
            let result = &v.element as *const _ as *mut Element;
            unsafe { &mut *result }
        })
    }

    pub fn sort<F>(&mut self, compare: F)
    where
        F: Fn(&Element, &Element) -> Ordering,
    {
        if self.is_sorted {
            self.insertion_sort(compare);
        } else {
            // TODO opt
            self.region_sort_result.truncate(0);
            for v in &self.elements {
                let id = v.element.id;
                self.region_sort_result.push(id);
            }
            self.quick_sort(compare);
            // turn off quick sort
            self.is_sorted = true;
        }
    }
}

impl Index<usize> for &mut ElementStore {
    type Output = Element;

    fn index(&self, index: usize) -> &Self::Output {
        let id = self.region_sort_result[index];
        self.get_element_by_id(id).unwrap()
    }
}

impl IndexMut<usize> for &mut ElementStore {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let id = self.region_sort_result[index];
        self.get_mut_element_by_id(id).unwrap()
    }
}

impl CollisionalCollection for &mut ElementStore {
    type Element = Element;
    fn len(&self) -> usize {
        self.region_sort_result.len()
    }

    fn sort(&mut self, compare: impl Fn(&Element, &Element) -> Ordering) {
        ElementStore::sort(self, compare)
    }
}
