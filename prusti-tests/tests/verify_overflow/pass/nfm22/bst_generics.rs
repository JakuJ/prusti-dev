// Requires PR #882

use prusti_contracts::*;
use std::cmp::{Ord, Ordering::{self, Equal, Less, Greater}};

fn main() {
    let value = 0;
    let bst = Tree::Node(value, Box::new(Tree::Empty), Box::new(Tree::Empty));
    // TODO: make this work:
    // assert!(bst.contains(&value));
}

// TODO 0: Would be nice to get invariants working!
pub enum Tree<T: Ord> {
    Node(T, Box<Tree<T>>, Box<Tree<T>>),
    Empty,
}

#[extern_spec]
trait Ord {
    #[pure]
    // TODO: move `compare` spec here
    fn cmp(&self, other: &Self) -> Ordering;
}

// Issue 1: could not use `a.cmp(other)` directly within a `forall`
// as it would complain about `cmp` not being pure. May be related to issue #4?
#[pure]
#[trusted]
#[ensures(match (result, compare(b, a)) {
    (Equal, Equal) |
    (Less, Greater) |
    (Greater, Less) => true,
    _ => false,
})]
#[ensures(forall(|c: &T| match (result, compare(b, c), compare(a, c)) {
    (Equal, Equal, Equal) => true,
    (Equal, Less, Less) => true,
    (Equal, Greater, Greater) => true,
    (Less, Equal, Less) => true,
    (Less, Less, Less) => true,
    (Less, Greater, _) => true,
    (Greater, Equal, Greater) => true,
    (Greater, Less, _) => true,
    (Greater, Greater, Greater) => true,
    _ => false,
}))]
fn compare<T: Ord>(a: &T, b: &T) -> Ordering {
    a.cmp(b)
}

impl<T: Ord> Tree<T> {
    #[pure]
    pub fn contains(&self, find_value: &T) -> bool {
        if let Tree::Node(value, left, right) = self {
            match compare(find_value, value) {
                Equal => true,
                Less => left.contains(find_value),
                Greater => right.contains(find_value),
            }
        } else { false }
    }
    predicate! {
    pub fn bst_invariant(&self) -> bool {
        if let Tree::Node(value, left, right) = self {
            forall(|i: T| left.contains(&i) == (matches!(compare(&i, value), Less) && self.contains(&i))) &&
            forall(|i: T| right.contains(&i) == (matches!(compare(&i, value), Greater) && self.contains(&i))) &&
            left.bst_invariant() && right.bst_invariant()
        } else { true }
    }
    }

    #[requires(matches!(self, Tree::Node(..)))]
    #[requires(self.bst_invariant())]
    #[assert_on_expiry(
        // Must hold before result can expire
        if let Tree::Node(_, left, right) = old(self) {
            forall(|i: T| left.contains(&i) == (matches!(compare(&i, result), Less) && old(self).contains(&i))) &&
            forall(|i: T| right.contains(&i) == (matches!(compare(&i, result), Greater) && old(self).contains(&i)))
        } else { false },
        // A postcondition of `get_root` after result expires
        self.bst_invariant() && self.contains(before_expiry(result))
        && (if let Tree::Node(value, _, _) = old(self) {
            (!matches!(compare(before_expiry(result), value), Equal) ==> !self.contains(value))
        } else { false })
    )]
    pub fn get_root(&mut self) -> &mut T {
        if let Tree::Node(value, _, _) = self { value } else { panic!() }
    }

    // Note: the invariant is not actually needed:
    #[requires(self.bst_invariant())]
    #[ensures(self.bst_invariant())]
    // Issue 5: `new_value` isn't automatically wrapped in `old(...)` (due to snapshot encoding imo)
    #[ensures(self.contains(old(&new_value)))]
    #[ensures(forall(|i: &T| !matches!(compare(old(&new_value), i), Equal) ==> old(self).contains(i) == self.contains(i)))]
    pub fn insert(&mut self, new_value: T) {
        if let Tree::Node(value, left, right) = self {
            match compare(&new_value, value) {
                Equal => (),
                Less => left.insert(new_value),
                Greater => right.insert(new_value),
            }
        } else {
            *self = Tree::Node(new_value, Box::new(Tree::Empty), Box::new(Tree::Empty))
        }
    }
}
