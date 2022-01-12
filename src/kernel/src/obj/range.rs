use alloc::sync::Arc;
use nonoverlapping_interval_tree::NonOverlappingIntervalTree;

use crate::mutex::Mutex;

use super::{
    pages::{Page, PageRef},
    pagevec::{PageVec, PageVecRef},
    PageNumber,
};

pub struct Range {
    start: PageNumber,
    length: usize,
    offset: usize,
    pv: PageVecRef,
}

impl Range {
    fn new(start: PageNumber) -> Self {
        Self {
            start,
            length: 0,
            offset: 0,
            pv: Arc::new(Mutex::new(PageVec::new())),
        }
    }

    fn get_page(&self, pn: PageNumber) -> PageRef {
        assert!(pn >= self.start);
        let off = pn - self.start;
        self.pv.lock().get_page(self.offset + off)
    }

    fn add_page(&self, pn: PageNumber, page: Page) {
        assert!(pn >= self.start);
        let off = pn - self.start;
        self.pv.lock().add_page(self.offset + off, page);
    }
}

pub struct RangeTree {
    tree: NonOverlappingIntervalTree<PageNumber, Range>,
}

impl RangeTree {
    pub fn get(&self, pn: PageNumber) -> Option<&Range> {
        self.tree.get(&pn)
    }

    pub fn get_mut(&mut self, pn: PageNumber) -> Option<&mut Range> {
        self.tree.get_mut(&pn)
    }

    pub fn get_page(&self, pn: PageNumber) -> Option<PageRef> {
        let range = self.get(pn)?;
        Some(range.get_page(pn))
    }

    pub fn add_page(&mut self, pn: PageNumber, page: Page) {
        let range = self.tree.get(&pn);
        if let Some(range) = range {
            range.add_page(pn, page);
        } else {
            let range = Range::new(pn);
            range.add_page(pn, page);
            self.tree.insert_replace(pn..pn.next(), range);
        }
    }
}
