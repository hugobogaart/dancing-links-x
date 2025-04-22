use itertools::{enumerate, Itertools};
use std::hint;

// 10k runs with safe ~ 90us, without safety ~75 us.
const UNSAFE_INDEXING: bool = true;
const UNSAFE_ENUM_ACCESS: bool = true;


// force removes all elements of rms from xs.
// Panics if one rm is not present.
fn force_remove_all <T: Eq> (xs: &mut Vec<T>, rms: &Vec<T>)
{
        for rm in rms {
                let opt_pos = xs.iter().position(|x| x == rm);
                let Some (pos) = opt_pos else {
                        panic!("rm element not found!");
                };
                xs.remove(pos);
        }
}

// Todo it's probably easier to store indices to everything instead of pointers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct NodeData {
        h_idx:  usize,
        row:    usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct HeaderData {
        n_col:  usize,   // Keeps track of the number of nodes in the column.
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EitherData {
        Node(NodeData),
        Header(HeaderData)
}

impl EitherData {
        fn force_ndat_mut (&mut self) -> &mut NodeData
        {
                if UNSAFE_ENUM_ACCESS {
                        unsafe{
                        match self {
                                Self::Node (ndat) => ndat,
                                _                 => std::hint::unreachable_unchecked(),
                        }
                        }
                } else {
                        match self {
                                Self::Node (ndat) => ndat,
                                _                 => panic!("force_ndat_mut called on a header!")
                        }
                }
        }

        fn force_hdat_mut (&mut self) -> &mut HeaderData
        {
                if UNSAFE_ENUM_ACCESS {
                        unsafe{
                        match self {
                                Self::Header (hdat) => hdat,
                                _                   => std::hint::unreachable_unchecked(),
                        }
                        }
                } else {
                        match self {
                                Self::Header (hdat) => hdat,
                                _                   => panic!("force_hdat_mut called on a node!")
                        }
                }
        }

        fn force_ndat (&self) -> &NodeData
        {
                if UNSAFE_ENUM_ACCESS {
                        unsafe{
                        match self {
                                Self::Node (ndat) => ndat,
                                _                 => std::hint::unreachable_unchecked(),
                        }
                        }
                } else {
                        match self {
                                Self::Node (ndat) => ndat,
                                _                 => panic!("force_ndat called on a header!")
                        }
                }
        }

        fn force_hdat (&self) -> &HeaderData
        {
                if UNSAFE_ENUM_ACCESS {
                        unsafe{
                        match self {
                                Self::Header (hdat) => hdat,
                                _                   => std::hint::unreachable_unchecked(),
                        }
                        }
                } else {
                        match self {
                                Self::Header (hdat) => hdat,
                                _                   => panic!("force_hdat called on a node!")
                        }
                }
        }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Node {
        u_idx:  usize,
        d_idx:  usize,
        l_idx:  usize,  // to node
        r_idx:  usize,  // to node

        spec_dat: EitherData,
}


impl Node {
        fn is_node (&self) -> bool
        {
                match self.spec_dat {
                        EitherData::Node(_)     => true,
                        _                       => false,
                }
        }
        fn is_header (&self) -> bool
        {
                match self.spec_dat {
                        EitherData::Header(_)   => true,
                        _                       => false,
                }
        }
}

pub
struct DancingLinkArray <'a, R: Eq> {

        nodes: Box<[Node]>,

        num_headers: usize,

        // We save references to rowdata for convenience of returning solutions.
        rows: Box<[&'a R]>,

        // We keep track of the currently active headers.
        // contains indices into the headers.
        active_headers: Vec<usize>,

        // Removed headers, where the last one is the most recently removed.
        removed_groups: Vec<(&'a R, Vec<usize>)>,
}

// Checks if a slice contains any contiguous duplicate elements.
fn sorted_unique <X, F: Fn(&X, &X) -> bool> (xs: &[X], f: F) -> bool
{
        let mut last: Option<&X> = None;
        for x in xs {
                if let Some (last_x) = last {
                        if f(last_x, x) {
                                return false;
                        }
                }
                last = Some (x);
        }
        true
}

impl <'a, R: Eq> DancingLinkArray <'a, R> {

        // Simplified factory.
        // Input must be sorted, and unique.
        // Not exported, to enforce these assumptions.
        fn from_idx_array (idc: &[(usize, usize)], num_cols: usize, rows: &[&'a R]) -> Option<Self>
        {
                let num_headers = num_cols;
                let num_rows = rows.len();
                // let num_nodes = idc.len();

                let empty_hdat = EitherData::Header(HeaderData {n_col: 0});

                let empty_header = Node {
                        u_idx: 0,
                        d_idx: 0,
                        r_idx: 0,
                        l_idx: 0,
                        spec_dat: empty_hdat
                };

                let make_empty_node = |r_idx: usize| {
                        let empty_ndat = EitherData::Node(NodeData {h_idx: 0, row: r_idx});
                        Node {
                                u_idx: 0,
                                d_idx: 0,
                                r_idx: 0,
                                l_idx: 0,
                                spec_dat: empty_ndat
                        }
                };

                // First we generate the headers.
                let headers_generating_it = std::iter::repeat_n(empty_header, num_headers);
                let node_generating_it = idc
                        .iter()
                        .map(|(r_idx, _)| make_empty_node(*r_idx));

                let mut nodes_box: Box<[Node]> = headers_generating_it.chain(node_generating_it).collect();

                // We access headers through "headers", nodes through "nodes".
                let (headers, nodes) = nodes_box.split_at_mut(num_headers);

                // Now we make the cyclic horizontal structure on the headers.
                for i in 0..(num_headers - 1) {
                        headers[i].r_idx = i + 1;
                        headers[i + 1].l_idx = i;
                }

                // And the end points connect.
                if num_headers >= 1 {
                        headers[0].l_idx = num_headers - 1;
                        headers[num_headers - 1].r_idx = 0;
                } else {
                        panic!("There must be at least one column!");
                }

                // And we do the same for each normal row.
                // Yes, this could be more efficient, since idc is already sorted.
                let mut tmp_idc: Vec<usize> = Vec::new();
                for r in 0..num_rows {
                        tmp_idc.clear();
                        // We find each node with this row.
                        for (n_idx, &(r_, _)) in idc.iter().enumerate() {
                                if r_ == r {
                                        tmp_idc.push(n_idx);
                                }
                        }

                        // tmp_idc contains the indices of each node in this row.
                        // We zip the row together just like we did the headers.
                        let num_in_row = tmp_idc.len();
                        for i in 0..(num_in_row - 1) {
                                let l_idx = tmp_idc[i];
                                let r_idx = tmp_idc[i + 1];
                                nodes[l_idx].r_idx = r_idx + num_headers;
                                nodes[r_idx].l_idx = l_idx + num_headers;
                        }
                        // And we make the cycle complete.
                        if num_in_row >= 1 {
                                let l_idx = tmp_idc[0];
                                let r_idx = tmp_idc[num_in_row - 1];
                                nodes[l_idx].l_idx = r_idx + num_headers;
                                nodes[r_idx].r_idx = l_idx + num_headers;
                        } else {
                                panic!("There should be a node in the row!");
                        }
                }

                // And we do the same for each column.

                for c in 0..num_cols{
                        tmp_idc.clear();
                        // We find each node with this column.
                        for (n_idx, &(_, c_)) in idc.iter().enumerate() {
                                if c_ == c {
                                        tmp_idc.push(n_idx);
                                }
                        }

                        // tmp_idc contains each node in the column.
                        let num_in_col = tmp_idc.len();
                        for i in 0..(num_in_col - 1) {
                                let u_idx = tmp_idc[i];
                                let d_idx = tmp_idc[i + 1];
                                nodes[u_idx].d_idx = d_idx + num_headers;
                                nodes[d_idx].u_idx = u_idx + num_headers;
                        }

                        // And for each column we have to make the cycle complete by sticking
                        // the header between.
                        if num_in_col >= 1 {
                                let u_idx = tmp_idc[0];
                                let d_idx = tmp_idc[num_in_col - 1];
                                let h_idx = c;

                                nodes[u_idx].u_idx = h_idx;
                                nodes[d_idx].d_idx = h_idx;
                                headers[h_idx].d_idx = u_idx + num_headers;
                                headers[h_idx].u_idx = d_idx + num_headers;

                        } else {
                                // Empty column given.
                                panic!("empty column");
                        }
                }

                // The cyclic row and column structure is now complete.
                // The only thing left to do is give each node a pointer to its header.
                for (n_idx, &(_, c)) in idc.iter().enumerate() {
                        nodes[n_idx].spec_dat.force_ndat_mut().h_idx = c;
                        headers[c].spec_dat.force_hdat_mut().n_col += 1;
                }

                let vec_to_active_headers = (0..num_headers).collect();

                // The structure is now complete.
                let box_rows: Box<[&'a R]> = Box::from(rows);

                Some (DancingLinkArray {
                        nodes: nodes_box,
                        num_headers,
                        rows: box_rows,
                        active_headers: vec_to_active_headers,
                        removed_groups: Vec::new()
                })
        }

        // Extremely general way of construction.
        // Yes, it's a mess.
        pub fn from_iter_general<I, OR, OC, FRef, FR, FC> (it: I, iref: FRef, i2r: FR, i2c: FC) -> Option<Self>
        where I: Iterator,
              OR: Ord,
              OC: Ord,
              FRef: Fn (&I::Item) -> &'a R,
              FR: Fn(&I::Item) -> OR,
              FC: Fn(&I::Item) -> OC
        {
                // We turn this mess of a factory into a vector of (&R) and (row_idx, col_idx).
                let nodes: Box<[(&'a R, OR, OC)]> = it.map(|i| (iref(&i), i2r(&i), i2c(&i))).collect();

                // We extract both the row and column data.
                let sorted_unique_rows: Box<[(&'a R, &OR)]> = {
                        let mut unique_rows: Box<[(&'a R, &OR)]> = nodes
                                .iter()
                                .map(|(rf, r, _)| (*rf, r))
                                .collect();
                        unique_rows.sort_unstable_by(|(_, r1), (_, r2)| r1.cmp(r2));
                        unique_rows
                                .into_iter()
                                .dedup_by(|(_, r1), (_, r2)| r1 == r2)
                                .collect()
                };

                // Extracting just the references.
                let sorted_row: Vec<&'a R> = sorted_unique_rows.iter().map(|(rf, _)| *rf).collect();

                // sorted unique columns
                let sorted_unique_cols: Vec<& OC> = {
                        let mut unique_cols: Vec<& OC> = nodes
                                .iter()
                                .map(|(_, _, c)| c)
                                .collect();
                        unique_cols.sort();
                        unique_cols
                                .into_iter()
                                .dedup()
                                .collect()
                };

                let num_cols = sorted_unique_cols.len();

                // To find indices, for each node we find for what row and column we have the index.
                let mut idc: Vec<(usize, usize)> = Vec::new();

                for (_, node_r, node_c) in &nodes {
                        let opt_r = sorted_unique_rows.iter().zip(0usize..).find(|((_, r), _)| **r == *node_r);
                        let opt_c = sorted_unique_cols.iter().zip(0usize..).find(|(c, _)| ***c == *node_c);
                        if let (Some((_, r_idx)), Some((_, c_idx))) = (opt_r, opt_c) {
                                idc.push((r_idx, c_idx));
                        } else {
                                panic!("Node no column or row");
                        }
                }

                // Now we sort the indices
                idc.sort_by(|(r1, c1), (r2, c2)| {
                        let rcmp = r1.cmp(r2);
                        if rcmp.is_eq() {
                                c1.cmp(c2)
                        } else {
                                rcmp
                        }
                });
                let unique = sorted_unique(idc.as_slice(), |l, r| l == r);
                if !unique {
                        return None;
                }

                Self::from_idx_array(idc.as_slice(), num_cols, sorted_row.as_slice())
        }


        // given slices of rows and column and a predicate, constructs array with
        // a node on each combination determined by the predicate.
        pub fn from_pred <C, Pred> (rows: &'a [R], cols: &[C], p: Pred) -> Option<Self>
        where
                Pred: Fn (&R, &C) -> bool,
        {
                let mut node_idc: Vec<(usize, usize)> = Vec::new();
                let row_refs: Vec<&'a R> = rows.iter().collect();

                for (row, r) in rows.iter().zip(0usize..) {
                        for (col, c) in cols.iter().zip(0usize..) {
                                if p(row, col) {
                                        node_idc.push((r, c));
                                }
                        }
                }
                Self::from_idx_array(node_idc.as_slice(), cols.len(), row_refs.as_slice())
        }

        fn header_is_active (&self, h_idx: usize) -> bool
        {
                self.active_headers.contains(&h_idx)
        }

        // Some node-graph functions.

        fn detach_node_vertical (&mut self, n_idx: usize)
        {
                let nodes = &mut self.nodes;

                if UNSAFE_INDEXING {
                        unsafe {
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).u_idx).d_idx = nodes.get_unchecked(n_idx).d_idx;
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).d_idx).u_idx = nodes.get_unchecked(n_idx).u_idx;
                        }
                } else {
                        nodes[nodes[n_idx].u_idx].d_idx = nodes[n_idx].d_idx;
                        nodes[nodes[n_idx].d_idx].u_idx = nodes[n_idx].u_idx;
                }
        }

        fn detach_node_horizontal (&mut self, n_idx: usize)
        {
                let nodes = &mut self.nodes;

                if UNSAFE_INDEXING {
                        unsafe {
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).l_idx).r_idx = nodes.get_unchecked(n_idx).r_idx;
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).r_idx).l_idx = nodes.get_unchecked(n_idx).l_idx;
                        }
                } else {
                        nodes[nodes[n_idx].l_idx].r_idx = nodes[n_idx].r_idx;
                        nodes[nodes[n_idx].r_idx].l_idx = nodes[n_idx].l_idx;
                }
        }

        fn insert_node_vertical (&mut self, n_idx: usize)
        {
                let nodes = &mut self.nodes;
                if UNSAFE_INDEXING {
                        unsafe {
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).u_idx).d_idx = n_idx;
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).d_idx).u_idx = n_idx;
                        }
                } else {
                        nodes[nodes[n_idx].u_idx].d_idx = n_idx;
                        nodes[nodes[n_idx].d_idx].u_idx = n_idx;
                }
        }

        fn insert_node_horizontal (&mut self, n_idx: usize)
        {
                let nodes = &mut self.nodes;
                if UNSAFE_INDEXING {
                        unsafe {
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).l_idx).r_idx = n_idx;
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).r_idx).l_idx = n_idx;
                        }
                } else {
                        nodes[nodes[n_idx].l_idx].r_idx = n_idx;
                        nodes[nodes[n_idx].r_idx].l_idx = n_idx;
                }
        }

        fn associated_header_index (&self, n_idx: usize) -> usize
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(n_idx).spec_dat.force_ndat().h_idx
                        }
                } else {
                        self.nodes[n_idx].spec_dat.force_ndat().h_idx
                }
        }

        fn associated_row_index (&self, n_idx: usize) -> usize
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(n_idx).spec_dat.force_ndat().row
                        }
                } else {
                        self.nodes[n_idx].spec_dat.force_ndat().row
                }
        }

        fn is_header_idx (&self, idx: usize) -> bool
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(idx).is_header()
                        }
                } else {
                        self.nodes[idx].is_header()
                }

        }

        fn is_node_idx (&self, idx: usize) -> bool
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(idx).is_node()
                        }
                } else {
                        self.nodes[idx].is_node()
                }
        }

        fn to_bottom (&self, idx: usize) -> usize
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(idx).d_idx
                        }
                } else {
                        self.nodes[idx].d_idx
                }
        }

        fn to_right (&self, idx: usize) -> usize
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(idx).r_idx
                        }
                } else {
                        self.nodes[idx].r_idx
                }
        }

        fn headers_colcount_mut (&mut self, h_idx: usize) -> &mut usize
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        &mut self.nodes.get_unchecked_mut(h_idx).spec_dat.force_hdat_mut().n_col
                        }
                } else {
                        &mut self.nodes[h_idx].spec_dat.force_hdat_mut().n_col
                }
        }

        fn headers_colcount (&self, h_idx: usize) -> usize
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(h_idx).spec_dat.force_hdat().n_col
                        }
                } else {
                        self.nodes[h_idx].spec_dat.force_hdat().n_col
                }
        }

        fn nodes_header_count_mut (&mut self, n_idx: usize) -> &mut usize
        {
                let h_idx = self.associated_header_index(n_idx);
                self.headers_colcount_mut(h_idx)
        }

        // Will remove the header from the structure
        // and place it in the removed headers vector.
        // we do NOT change the self.activeheader and self.removed_header fields.
        fn remove_header (&mut self, h_idx: usize)
        {
                debug_assert!(self.nodes[h_idx].is_header());
                // We assert the header is active, and not removed.
                debug_assert!(self.header_is_active(h_idx));
                debug_assert!(!self.removed_groups.iter().any(|(_, g)| g.iter().contains(&h_idx)));

                // First we remove the header from the nodes.
                self.detach_node_horizontal(h_idx);
                self.detach_node_vertical(h_idx);

                // If there are no nodes in the column, we're done.
                // Otherwise for each node in the column we remove the row.
                let entry_idx = self.to_bottom(h_idx);
                if self.is_header_idx(entry_idx) {
                        return
                }

                let mut v_nidx = entry_idx;
                loop {
                        // For this node, we remove its row from their columns.
                        let mut h_nidx = self.to_right(v_nidx);
                        while h_nidx != v_nidx {

                                self.detach_node_vertical(h_nidx);
                                *self.nodes_header_count_mut(h_nidx) -= 1;

                                // increment horizontally.
                                h_nidx = self.to_right(h_nidx);
                        }

                        // increment vertically.
                        v_nidx = self.to_bottom(v_nidx);
                        if v_nidx == entry_idx {
                                break;
                        }
                }
        }

        // unsafely restores header that was previously removed with remove_header.
        fn restore_header (&mut self, h_idx: usize)
        {
                debug_assert!(self.nodes[h_idx].is_header());
                // We assert the header is inactive, and removed.
                debug_assert!(!self.header_is_active(h_idx));
                // debug_assert!(self.removed_groups.iter().any(|(_, g)| g.iter().contains(&h_idx)));

                // If there are nodes in the column, we restore them.
                let entry_nidx = self.to_bottom(h_idx);

                if self.is_node_idx (entry_nidx) {

                        let mut v_nidx = entry_nidx;
                        loop {
                                let mut h_nidx = self.to_right(v_nidx);
                                while h_nidx != v_nidx {
                                        self.insert_node_vertical(h_nidx);
                                        *self.nodes_header_count_mut(h_nidx) += 1;

                                        // increment horizontally.
                                        h_nidx = self.to_right(h_nidx);
                                }

                                // increment vertically.
                                v_nidx = self.to_bottom(v_nidx);
                                if v_nidx == entry_nidx {
                                        break;
                                }
                        }

                }

                // Now we restore the header.
                self.insert_node_vertical(h_idx);
                self.insert_node_horizontal(h_idx);
        }

        // Removes header for entire row. Equivelent to "making a choice".
        fn remove_row(&mut self, repr_idx: usize)
        {
                debug_assert!(self.is_node_idx(repr_idx));

                let rref: &'a R = {
                        if UNSAFE_INDEXING {
                                unsafe{
                                self.rows.get_unchecked_mut(self.associated_row_index(repr_idx))
                                }
                        } else {
                                self.rows[self.associated_row_index(repr_idx)]
                        }
                };

                // We collect the headers of all columns where our row intersects.
                // The order is that of removal.

                // These are the indices of the headers we want to remove
                // in the self.headers array.

                let mut headers_idc: Vec<usize> = Vec::new();
                let mut h_nidx = repr_idx;
                loop {
                        debug_assert!(self.is_node_idx(h_nidx));

                        headers_idc.push(self.associated_header_index(h_nidx));

                        h_nidx = self.to_right(h_nidx);
                        if h_nidx == repr_idx {
                                break;
                        }
                }

                // And we actually remove them.
                for &h_idx in &headers_idc {
                        self.remove_header(h_idx);
                }

                // If the indices of the headers we want to remove
                // are not in self.active_headers, we panic.
                force_remove_all(&mut self.active_headers, &headers_idc);

                // And we save the indices of these removed headers together in a group.
                self.removed_groups.push((rref, headers_idc));
        }

        // Restores row after it was popped.
        fn restore_row(&mut self)
        {
                let Some((_, removed_group)) = self.removed_groups.pop() else {
                        panic!("Tried to restore non existing row!")
                };

                for &h_idx in removed_group.iter().rev() {
                        self.restore_header(h_idx);
                        self.active_headers.push(h_idx);
                }
        }

        fn solve_one (&mut self) -> Option <Vec<usize>>
        {
                // (h_idx, n_col) pair with smallest n_col.
                let optminpair = self.active_headers.iter()
                        .map(|h_idx| (h_idx, self.headers_colcount(*h_idx)))
                        .reduce(|l@(_, n1), r@(_, n2)| if n1 < n2 {l} else {r});

                let Some ((lowest_count_header_idx, _)) = optminpair else {
                        // We're done, there are no more columns.
                        return Some (Vec::new());
                };

                // We iterate through the column and if removing any node solves
                // the array, we're done.

                let mut v_nidx = self.to_bottom(*lowest_count_header_idx);
                while self.is_node_idx(v_nidx) {

                        self.remove_row(v_nidx);
                        let opt_subsol = self.solve_one();
                        self.restore_row();

                        if let Some (mut subsol) = opt_subsol {
                                let r = self.associated_row_index(v_nidx);
                                subsol.push(r);
                                return Some(subsol);
                        }
                        v_nidx = self.to_bottom(v_nidx);
                }

                // No row yields a solution.
                None
        }

        fn solve_many (&mut self) -> Vec <Vec<usize>>
        {
                // (h_idx, n_col) pair with smallest n_col.
                let optminpair = self.active_headers.iter()
                        .map(|h_idx| (h_idx, self.headers_colcount(*h_idx)))
                        .reduce(|l@(_, n1), r@(_, n2)| if n1 < n2 {l} else {r});

                let Some ((lowest_count_header_idx, _)) = optminpair else {
                        // We're done, there are no more columns.
                        return vec!(Vec::new());
                };

                let mut all_solutions: Vec<Vec<usize>> = Vec::new();

                let mut v_nidx = self.to_bottom(*lowest_count_header_idx);
                while self.is_node_idx(v_nidx) {

                        // We only consider this node if it can't lead
                        // to a permutation of solution we already have.

                        // Specifically, we don't run this row if one of the
                        // subsolutions already has this index

                        let r_idx = self.associated_row_index(v_nidx);

                        let skip_row = all_solutions.iter().any(|subsol| subsol.contains(&r_idx));
                        if skip_row {
                                continue;
                        }

                        self.remove_row(v_nidx);
                        let subsols = self.solve_many();
                        self.restore_row();


                        for mut subsol in subsols {
                                subsol.push(r_idx);
                                all_solutions.push(subsol);
                        }

                        v_nidx = self.to_bottom(v_nidx);
                }

                all_solutions
        }

        pub fn solve_one_ref (&mut self) -> Option <Vec<&'a R>>
        {
                self.solve_one().map(|v| v.iter().map(|r_idx| self.rows[*r_idx]).collect())
        }

        pub fn solve_many_ref (&mut self) -> Vec <Vec<&'a R>>
        {
                self.solve_many()
                        .iter()
                        .map(|v| v.iter()
                            .map(|r_idx| self.rows[*r_idx])
                            .collect())
                        .collect()
        }

        // Will find an active node in each row, unless it doesn't exist.
        fn get_node_per_row (&self) -> Box<[Option<usize>]>
        {
                let num_rows = self.rows.len();

                // Saves the index of any node that has this row index.
                let mut found_indices: Box<[Option<usize>]> = std::iter::repeat_n(None, num_rows).collect();

                let num_nodes = self.nodes.len();
                for n_idx in self.num_headers .. num_nodes {
                        let r_idx = self.associated_row_index(n_idx);
                        let found_index = &mut found_indices[r_idx];
                        if found_index.is_none() {
                                *found_index = Some (n_idx);
                        }
                }

                // For each node who's header is inactive, all nodes in that row are deleted.
                for opt_nidx in &mut found_indices {
                        if let Some (n_idx) = opt_nidx {
                                let h_idx = self.associated_header_index(*n_idx);
                                if !self.header_is_active(h_idx) {
                                        *opt_nidx = None;
                                }
                        }
                }

                // Now we only have non deleted rows.
                found_indices
        }

        // Makes choices given row indices.
        pub fn make_choices <I: Iterator<Item=usize>> (&mut self, rows: I)
        {
                // First we find representing nodes.
                let mut rm_idc: Vec<usize> = Vec::new();

                let num_rows = self.rows.len();
                let mut visited: Box<[bool]> = std::iter::repeat_n(false, num_rows).collect();
                let row_nodes = self.get_node_per_row();

                for r_idx in rows {
                        if r_idx >= num_rows {
                                panic!("Row index out of bounds!");
                        }
                        if visited[r_idx] {
                                panic!("Row already removed!");
                        }
                        visited[r_idx] = true;

                        let Some(repr_idx) = row_nodes[r_idx] else {
                                panic!("This row has no nodes to remove!");
                        };

                        rm_idc.push(repr_idx);
                }

                // Each row now has a unique representing node.
                for rm_idx in rm_idc {
                        self.remove_row(rm_idx);
                }
        }

        // Version where the user gives gives references.
        // We find which rows correspond to the references.
        pub fn make_choices_ref <I: Iterator<Item=&'a R>> (&mut self, rows: I)
        {
                // For each reference we just find the index in self.rows.
                let opt_r_idc: Option<Vec<usize>> = rows.map(|rf| -> Option <usize> {
                                self.rows.iter()
                                        .position(|rrf| *rrf == rf)
                        }).collect();

                let Some (r_idc) = opt_r_idc else {
                        panic!("Given reference does not refer to any row!");
                };

                self.make_choices(r_idc.into_iter());
        }

        pub fn unmake_choices (&mut self, n_choices: usize)
        {
                for _ in 0usize..n_choices {
                        self.restore_row();
                }
        }
}
