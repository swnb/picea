use std::{collections::BTreeMap, rc::Rc};

use super::{Element, ID};

/**
 * ElementStore store all element with sort result cache
 */
pub struct ElementStore {
    elements: Vec<Rc<Element>>,          // origin element order
    element_sort_cache: Option<Vec<ID>>, // "sweep_and_prune_collision_detection" sort cache
    map: BTreeMap<ID, Rc<Element>>,      // indexMap find origin element index by element id
}

pub struct ElementIterMut {
    index: usize,
}

impl ElementStore {
    pub fn push(&mut self, element: Element) {
        let element = Rc::new(element);
        self.elements.push(element.clone());
        self.map.insert(element.id, element);
    }

    pub fn get_element_by_id(&self, id: ID) -> Option<&Element> {
        self.map.get(&id).map(|v| &**v)
    }

    // fn iter_mut_after_sort(&mut self, element: &mut Element) -> impl Iterator<Item = &mut Element> {
    //     // if not initial, use quick sort
    //     if let Some(elements_after_quick_sort) = self.element_sort_cache.as_ref() {}

    //     todo!()
    //     // self.elements.iter_mut().map(|v| &mut **v)
    // }
}
