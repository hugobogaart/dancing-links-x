mod dancing_link_array_optional;

use dancing_link_array_optional as dla;
use itertools::Itertools;
use std::cmp::Ordering;

// Public interface to the DLA.

pub
struct UCSolver <R: Clone + Eq> {
        array: dla::DancingLinkArray,

        row_dat: Box<[R]>,

        // Keeps track of the state we put the array in since construction.
        // When removing a row from the DLA, we store a handle to it here,
        // and when restoring, we pop it.
        rm_rows: Vec<usize>,

        // We remember a single node for each row, for performance.
        // This is constructed at construction and does not change.
        to_rows: Box<[dla::NodeIdx]>
}


impl <R: Clone + Eq> UCSolver <R> {

        // The constructors:

        // Constructs a UCSolver by taking the rows, columns and
        // constructs a node there or not, depending on the given predicate.
        pub
        fn from_pred <C, P: Fn(&R, &C) -> bool> (rows: &[R], cols: &[C], p: P) -> UCSolver < R>
        {
                let row_dat: Box<[R]> = rows.iter().cloned().collect();

                let idc_gen = rows.iter().enumerate()
                                .cartesian_product(cols.iter().enumerate())
                        .filter (|((_, r), (_, c))| p(r, c))
                        .map (|((r_idx, _), (c_idx, _))| (r_idx, c_idx));

                let dla = dla::DancingLinkArray::from_sorted_idc_unsafe(idc_gen, rows.len(), cols.len(), 0);
                let to_rows = dla.to_each_row();

                UCSolver {array: dla, row_dat, rm_rows: Vec::new(), to_rows}
        }

        // Like from_pred, but distinguishes strict and optional columns.
        pub
        fn from_pred_opt <C, P: Fn(&R, &C) -> bool> (rows: &[R], strict_cols: &[C], opt_cols: &[C], p: P) -> UCSolver < R>
        {
                let row_dat: Box<[R]> = rows.iter().cloned().collect();

                let cols_it = strict_cols.iter().chain(opt_cols.iter());

                let idc_gen = rows.iter().enumerate()
                                .cartesian_product(cols_it.enumerate())
                        .filter (|((_, r), (_, c))| p(r, c))
                        .map (|((r_idx, _), (c_idx, _))| (r_idx, c_idx));

                let num_strict_cols = strict_cols.len();
                let num_opt_cols = opt_cols.len();
                let dla = dla::DancingLinkArray::from_sorted_idc_unsafe(idc_gen, rows.len(), num_strict_cols, num_opt_cols);

                let to_rows = dla.to_each_row();

                UCSolver {array: dla, row_dat, rm_rows: Vec::new(), to_rows}
        }

        // O(n^2).
        // Yeah, this could be faster for large n,
        // but it probably doesn't matter.
        //
        // Constructs a UCSolver by taking all the nodes.
        pub
        fn from_it <C: Eq, I: IntoIterator<Item = (R, C)>> (it: I) -> Option<UCSolver <R>>
        {
                let abstract_idc: Box<[(R, C)]> = it.into_iter().collect();
                let mut unique_rows:  Vec<R> = Vec::new();
                let mut unique_cols:  Vec<C> = Vec::new();
                let mut idc: Vec<(usize, usize)> = Vec::with_capacity(abstract_idc.len());

                for (r, c) in abstract_idc {
                        let opt_ridx = unique_rows.iter().position(|r_| *r_ == r);
                        let opt_cidx = unique_cols.iter().position(|c_| *c_ == c);

                        let row = match opt_ridx {
                                Some (r_idx)    => r_idx,
                                None            => {
                                        unique_rows.push(r);
                                        unique_rows.len() - 1
                                }
                        };
                        let col = match opt_cidx {
                                Some (c_idx)    => c_idx,
                                None            => {
                                        unique_cols.push(c);
                                        unique_cols.len() - 1
                                }
                        };
                        idc.push((row, col));
                }
                sort_idc_rowmaj (&mut idc);
                if !sorted_idc_unique(&idc) {
                        return None;
                }

                let num_rows = unique_rows.len();
                let num_cols = unique_cols.len();
                let dla = dla::DancingLinkArray::from_sorted_idc_unsafe(idc, num_rows, num_cols, 0);
                let to_rows = dla.to_each_row();
                let rm_rows = Vec::new();
                Some (UCSolver {array: dla, row_dat: unique_rows.into_boxed_slice(), rm_rows, to_rows})
        }

        // Returns a solution, if exists.
        pub
        fn solve_one (&mut self) -> Option<Vec<R>>
        {
                let idc = self.array.solve_one()?;
                Some (idc.into_iter().map(|idx| self.row_dat[idx as usize].clone()).collect())
        }

        // Returns all solutions.
        pub
        fn solve_many (&mut self) -> Vec<Vec<R>>
        {
                let sols = self.array.solve_many();
                sols.into_iter().map(|sol| {
                        sol.into_iter().map(|idx| self.row_dat[idx as usize].clone()).collect()
                }).collect()
        }

        // Applies one change to the board.
        // Also sets internal state.
        fn set_state1 (&mut self, r: &R)
        {
                // First we find the row-index corresponding to this particular given row.
                let opt_r_idx = self.row_dat.iter().position(|row| row == r);

                // Obviously, this row has to exist.
                let Some(r_idx) = opt_r_idx else {
                        panic! ("Tried to remove a non-existant row!");
                };

                // And this row has to be currently not removed.
                if self.rm_rows.contains(&r_idx) {
                        panic!("Tried to remove an already removed row!");
                }

                // We find some node in the dla that has this row, and remove it.
                // We also remember we removed this row.

                let node_entry: dla::NodeIdx = self.to_rows[r_idx];
                self.array.rm_row(node_entry);
                self.rm_rows.push(r_idx);
        }


        pub
        fn set_state <'b, I: IntoIterator<Item = &'b R>>(&mut self, r_it: I)
        where R: 'b
        {
                for r in r_it {
                        self.set_state1(r);
                }
        }

        // Recovers n changes, previously made with set_state.
        pub
        fn recover_n (&mut self, n: usize)
        {
                for _ in 0..n {
                        let r_idx = self.rm_rows.pop().expect("Tried to recover nonexistent change!");
                        let entry_node = self.to_rows[r_idx];
                        self.array.insert_row(entry_node);
                }
        }

        // Wrapper around set_state >> solve_one >> recover_n.
        pub
        fn solve_one_with <'b, I: IntoIterator<Item = &'b R>> (&mut self, r_it: I) -> Option<Vec<R>>
        where R: 'b
        {
                let mut cnt = 0;
                for r in r_it {
                        self.set_state1(r);
                        cnt += 1;
                }
                let sol = self.solve_one();
                self.recover_n(cnt);
                sol
        }

        // Wrapper around set_state >> solve_many >> recover_n.
        pub
        fn solve_many_with <'b, I: IntoIterator<Item = &'b R>> (&mut self, r_it: I) -> Vec<Vec<R>>
        where R: 'b
        {
                let mut cnt = 0;
                for r in r_it {
                        self.set_state1(r);
                        cnt += 1;
                }
                let sol = self.solve_many();
                self.recover_n(cnt);
                sol
        }
}

// sorts (row, col) inplace, row major.
fn sort_idc_rowmaj (idc: &mut [(usize, usize)])
{
        fn ord ((r1, c1): &(usize, usize), (r2, c2): &(usize, usize)) -> Ordering
        {
                let r_comp = r1.cmp(r2);
                if r_comp.is_eq() {
                        c1.cmp(c2)
                } else {
                        r_comp
                }
        }
        idc.sort_unstable_by(ord);
}

// Takes sorted elements, and returns wether all elements are unique.
fn sorted_idc_unique (idc: &[(usize, usize)]) -> bool
{
        let mut it = idc.iter();
        let Some(&(mut r, mut c)) = it.next() else {
                return true;
        };
        for (r_, c_) in it {
                if (*r_ == r) && (*c_ == c) {
                        return false;
                }
                r = *r_;
                c = *c_;
        }
        true
}
