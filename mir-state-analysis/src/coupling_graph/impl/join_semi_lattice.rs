// © 2023, ETH Zurich
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::hash_map::Entry;

use prusti_rustc_interface::dataflow::JoinSemiLattice;

use crate::{
    free_pcs::{
        CapabilityKind, CapabilityLocal, CapabilityProjections, CapabilitySummary, Fpcs, RepackOp,
    },
    utils::{PlaceOrdering, PlaceRepacker},
};

use super::cg::{Cg, Graph, SharedBorrows, PathCondition};

impl JoinSemiLattice for Cg<'_, '_> {
    #[tracing::instrument(name = "Cg::join", level = "debug", skip_all)]
    fn join(&mut self, other: &Self) -> bool {
        // if self.done == 20 {
        //     panic!();
        //     // return false;
        // }
        // self.done += 1;
        let actually_changed = self.regions.graph.join(&other.regions.graph);
        return actually_changed;
        let mut changed = self.live.union(&other.live);
        for (idx, data) in other.regions.borrows.iter() {
            match self.regions.borrows.entry(*idx) {
                Entry::Occupied(mut o) => {
                    let (a, b) = o.get_mut();
                    for r in &data.0 {
                        if !a.contains(r) {
                            changed = true;
                            a.push(*r);
                        }
                    }
                    for r in &data.1 {
                        if !b.contains(r) {
                            changed = true;
                            b.push(*r);
                        }
                    }
                }
                Entry::Vacant(v) => {
                    changed = true;
                    v.insert(data.clone());
                }
            }
        }
        for s in &other.regions.subset {
            if !self.regions.subset.contains(s) {
                changed = true;
                self.regions.subset.push(*s);
            }
        }
        // if self.regions.graph != other.regions.graph {
        //     changed = true;
        //     self.regions.graph = other.regions.graph.clone();
        // }
        actually_changed
    }
}

impl JoinSemiLattice for Graph<'_, '_> {
    #[tracing::instrument(name = "Graph::join", level = "debug", ret)]
    fn join(&mut self, other: &Self) -> bool {
        // println!("Joining graphs:\n{:?}: {:?}\n{:?}: {:?}", self.id, self.nodes, other.id, other.nodes);
        let mut changed = false;
        for node in other.nodes.iter().flat_map(|n| n) {
            // let rep = node.regions.iter().next().unwrap();
            let rep = node.region;
            for (to, reasons) in node.blocks.iter() {
                let (from, to) = other.edge_to_regions(node.id, *to);
                let was_new = self.outlives_many(to, from, reasons);
                changed = changed || was_new;
                // if was_new {
                //     println!("New edge: {:?} -> {:?} ({:?})", from, to, reasons);
                // }
            }
            if node.borrows_from_static {
                changed = changed || self.set_borrows_from_static(rep);
            }
        }
        let old_len = self.static_regions.len();
        for &r in &other.static_regions {
            self.make_static(r);
        }
        changed = changed || self.static_regions.len() != old_len;
        changed
    }
}

impl SharedBorrows<'_> {
    fn join(&mut self, other: &Self) -> bool {
        println!("Joining shared borrows: {:?} and {:?}", self.location_map, other.location_map);
        let old_len = self.location_map.len();
        for (k, v) in other.location_map.iter() {
            self.insert(*k, v.clone());
        }
        self.location_map.len() != old_len
    }
}

impl JoinSemiLattice for PathCondition {
    fn join(&mut self, other: &Self) -> bool {
        assert_eq!(self.bb, other.bb);
        let old_values = self.values;
        self.values |= other.values;
        let mut changed = old_values != self.values;
        for (i, v) in self.other_values.iter_mut().enumerate() {
            let old_value = *v;
            *v |= other.other_values[i];
            changed = changed || old_value != *v;
        }
        changed
    }
}
