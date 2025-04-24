use std::cmp::Ordering;

use itertools::Itertools;

const UNSAFE_INDEXING: bool = true;

pub
type NodeIdx = u32;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Node {
        u:      NodeIdx,
        d:      NodeIdx,
        l:      NodeIdx,
        r:      NodeIdx,

        // We Keep track of the row index.
        row:    NodeIdx,
        col:    NodeIdx
}

pub
struct DancingLinkArray {
        // 0th element is the root header.
        nodes: Box<[Node]>,

        // Size for each column.
        sizes: Box<[u64]>,

        // header_idc: Vec<NodeIdx>,

        // saves one node per row.
        // row_idc: Box<[NodeIdx]>,
}

impl DancingLinkArray {

        // Some utilities.
        fn get_it_over_node_idc (&self) -> impl Iterator<Item = NodeIdx>
        {
                let n_h = self.num_headers()    as NodeIdx;
                let n_all = self.nodes.len()    as NodeIdx;
                n_h .. n_all
        }

        fn rm_node_hor (&mut self, n_idx: NodeIdx)
        {
                let n_idx = n_idx as usize;
                let nodes = &mut self.nodes;
                if UNSAFE_INDEXING {
                        unsafe {
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).l as usize).r
                                = nodes.get_unchecked(n_idx).r;
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).r as usize).l
                                = nodes.get_unchecked(n_idx).l;
                        }
                } else {
                        nodes[nodes[n_idx].l as usize].r = nodes[n_idx].r;
                        nodes[nodes[n_idx].r as usize].l = nodes[n_idx].l;
                }

        }
        fn rm_node_ver (&mut self, n_idx: NodeIdx)
        {
                let n_idx = n_idx as usize;
                let nodes = &mut self.nodes;
                if UNSAFE_INDEXING {
                        unsafe {
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).u as usize).d
                                = nodes.get_unchecked(n_idx).d;
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx).d as usize).u
                                = nodes.get_unchecked(n_idx).u;
                        }
                } else {
                        nodes[nodes[n_idx].u as usize].d = nodes[n_idx].d;
                        nodes[nodes[n_idx].d as usize].u = nodes[n_idx].u;
                }
        }
        fn insert_node_hor (&mut self, n_idx: NodeIdx)
        {
                let nodes = &mut self.nodes;
                if UNSAFE_INDEXING {
                        unsafe {
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx as usize).l as usize).r = n_idx;
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx as usize).r as usize).l = n_idx;
                        }
                } else {
                        nodes[nodes[n_idx as usize].l as usize].r = n_idx;
                        nodes[nodes[n_idx as usize].r as usize].l = n_idx;
                }
        }
        fn insert_node_ver (&mut self, n_idx: NodeIdx)
        {
                let nodes = &mut self.nodes;
                if UNSAFE_INDEXING {
                        unsafe {
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx as usize).u as usize).d = n_idx;
                        nodes.get_unchecked_mut(nodes.get_unchecked(n_idx as usize).d as usize).u = n_idx;
                        }
                } else {
                        nodes[nodes[n_idx as usize].u as usize].d = n_idx;
                        nodes[nodes[n_idx as usize].d as usize].u = n_idx;
                }
        }

        fn to_bottom (&self, idx: NodeIdx) -> NodeIdx
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(idx as usize).d
                        }
                } else {
                        self.nodes[idx as usize].d
                }
        }
        fn to_left (&self, idx: NodeIdx) -> NodeIdx
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(idx as usize).l
                        }
                } else {
                        self.nodes[idx as usize].l
                }
        }
        fn to_right (&self, idx: NodeIdx) -> NodeIdx
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(idx as usize).r
                        }
                } else {
                        self.nodes[idx as usize].r
                }
        }
        fn to_header (&self, n_idx: NodeIdx) -> NodeIdx
        {
                // The first node in the self.nodes array
                // is root. Therefore, col 0 is indexed at 1.
                self.get_col(n_idx) + 1
        }

        fn get_col (&self, idx: NodeIdx) -> NodeIdx
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(idx as usize).col
                        }
                } else {
                        self.nodes[idx as usize].col
                }
        }
        fn get_row (&self, idx: NodeIdx) -> NodeIdx
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        self.nodes.get_unchecked(idx as usize).row
                        }
                } else {
                        self.nodes[idx as usize].row
                }
        }

        fn root (&self) -> NodeIdx
        {
                0
        }

        fn num_cols (&self) -> usize
        {
                self.sizes.len()
        }
        fn num_rows (&self) -> usize
        {
                self.nodes.last().map(|nd| (nd.row as usize) + 1).unwrap_or(0)
        }

        fn num_headers (&self) -> usize
        {
                // We count the root as well.
                self.num_cols() + 1
        }

        fn is_header (&self, idx: NodeIdx) -> bool
        {
                (idx as usize) < self.num_headers()
        }

        fn get_size_col (&self, col: NodeIdx) -> u64
        {
                if UNSAFE_INDEXING {
                        unsafe {
                        *self.sizes.get_unchecked(col as usize)
                        }
                } else {
                        self.sizes[col as usize]
                }
        }
        fn get_size_node (&self, idx: NodeIdx) -> u64
        {
                let col = self.get_col(idx);
                if UNSAFE_INDEXING {
                        unsafe {
                        *self.sizes.get_unchecked(col as usize)
                        }
                } else {
                        self.sizes[col as usize]
                }
        }

        fn get_size_node_mut (&mut self, idx: NodeIdx) -> &mut u64
        {
                let col = self.get_col(idx);
                if UNSAFE_INDEXING {
                        unsafe {
                        self.sizes.get_unchecked_mut(col as usize)
                        }
                } else {
                        &mut self.sizes[col as usize]
                }
        }

        fn cover_col (&mut self, c: NodeIdx)
        {
                debug_assert!(self.is_header(c));

                self.rm_node_hor(c);
                let mut v_idx = self.to_bottom(c);
                while v_idx != c {
                        // Now we remove this node's row from their column
                        let mut h_idx = self.to_right(v_idx);
                        while h_idx != v_idx {
                                self.rm_node_ver(h_idx);
                                *self.get_size_node_mut(h_idx) -= 1;
                                h_idx = self.to_right(h_idx);
                        }
                        v_idx = self.to_bottom(v_idx);
                }
        }

        fn uncover_col (&mut self, c: NodeIdx)
        {
                debug_assert!(self.is_header(c));

                let mut v_idx = self.to_bottom(c);
                while v_idx != c {
                        // Now we insert this node's row from their column
                        let mut h_idx = self.to_right(v_idx);
                        while h_idx != v_idx {
                                self.insert_node_ver(h_idx);
                                *self.get_size_node_mut(h_idx) += 1;
                                h_idx = self.to_right(h_idx);
                        }
                        v_idx = self.to_bottom(v_idx);
                }
                self.insert_node_hor(c);
        }

        // Covers each column in the row of n_idx
        // public, since the DLXsolver may want to manually remove rows.
        pub
        fn rm_row (&mut self, n_idx: NodeIdx)
        {
                let last_to_remove = self.to_left(n_idx);
                let mut hor_it_idx = n_idx;
                loop {
                        let c = self.to_header(hor_it_idx);
                        self.cover_col(c);
                        if hor_it_idx == last_to_remove {
                                break;
                        }
                        hor_it_idx = self.to_right(hor_it_idx);
                }
        }

        pub
        fn insert_row (&mut self, n_idx: NodeIdx)
        {
                // We have to insert the columns in the opposite
                // order in which we removed them.
                let entry_idx = self.to_left(n_idx);    // Was removed last
                let mut hor_it_idx = entry_idx;
                loop {
                        let c = self.to_header(hor_it_idx);
                        self.uncover_col(c);

                        hor_it_idx = self.to_left(hor_it_idx);
                        if hor_it_idx == entry_idx {
                                break;
                        }
                }
        }

        fn lowest_header (&self) -> Option <NodeIdx>
        {
                let mut h_idx = self.to_right(self.root());
                if h_idx == self.root() {
                        return None;
                }

                let mut lowest_idx   = h_idx;
                let mut lowest_count = self.get_size_node(h_idx);
                h_idx = self.to_right(h_idx);
                while h_idx != self.root () {
                        let count = self.get_size_node(h_idx);
                        if count < lowest_count {
                                lowest_count = count;
                                lowest_idx = h_idx;
                        }
                        h_idx = self.to_right(h_idx);
                }
                Some(lowest_idx)
        }


        /*
        fn with_highest_header (&self, n_idx: NodeIdx) -> NodeIdx
        {
                let col = self.get_col(n_idx);
                let mut current_best_sz = self.get_size(col);
                let mut current_best_idx = n_idx;
                let mut h_idx = self.to_right(n_idx);
                while h_idx != n_idx {
                        let h_col = self.get_col(h_idx);
                        let h_sz = self.get_size(h_col);
                        if h_sz > current_best_sz {
                                current_best_sz = h_sz;
                                current_best_idx = h_idx;
                        }
                        h_idx = self.to_right(h_idx);
                }
                current_best_idx
        }
        */

        pub
        fn solve_one (&mut self) -> Option <Vec<NodeIdx>>
        {
                // First we find the header with the lowest index.
                let Some(lowest_c) = self.lowest_header() else {
                        // No columns! We're done.
                        return Some(Vec::new());
                };

                let mut v_idx = self.to_bottom(lowest_c);
                while v_idx != lowest_c {

                        self.rm_row(v_idx);
                        let opt_sub_sol = self.solve_one();
                        self.insert_row(v_idx);

                        if let Some(mut sub_sol) = opt_sub_sol {
                                let r = self.get_row(v_idx);
                                sub_sol.push(r);
                                return Some(sub_sol);
                        }
                        v_idx = self.to_bottom(v_idx);
                }
                None
        }

        pub
        fn solve_many (&mut self) -> Vec <Vec<NodeIdx>>
        {
                // First we find the header with the lowest index.
                let Some(lowest_c) = self.lowest_header() else {
                        // No columns! We're done.
                        return vec!(Vec::new());
                };

                let mut sols: Vec <Vec<NodeIdx>> = Vec::new();

                let mut v_idx = self.to_bottom(lowest_c);
                while v_idx != lowest_c {
                        let r = self.get_row(v_idx);
                        // We only run the row if no current solution
                        // contains r.
                        let skip_r = sols.iter().any(|sol| sol.contains(&r));
                        if skip_r {
                                continue;
                        }

                        self.rm_row(v_idx);
                        let sub_sols = self.solve_many();
                        self.insert_row(v_idx);

                        // We add each subsolution to the solutions, after inserting this row.
                        for mut sub_sol in sub_sols {
                                sub_sol.push(r);
                                sols.push(sub_sol);
                        }
                        v_idx = self.to_bottom(v_idx);
                }
                sols
        }

        // Assumes the elements are sorted row-major and unique.
        pub
        fn construct_from_sorted_unsafe <I> (elems_gen: I, num_rows: usize, num_cols: usize) -> DancingLinkArray
        where
                I: IntoIterator<Item = (usize, usize)>
        {
                fn gen_header (col: usize) -> Node
                {
                        Node {
                                u: 0,
                                d: 0,
                                l: 0,
                                r: 0,
                                row: 0, // Meaningless.
                                col: col as NodeIdx,
                        }
                }

                fn gen_node ((row, col): (usize, usize)) -> Node
                {
                        Node {
                                u: 0,
                                d: 0,
                                l: 0,
                                r: 0,
                                row: row as NodeIdx,
                                col: col as NodeIdx,
                        }
                }

                // We assume elems is sorted.
                let gen_root    = std::iter::once(gen_header(0));
                let gen_headers = (0..num_cols).map(gen_header);
                let gen_nodes   = elems_gen.into_iter().map (gen_node);

                let mut nodes: Box<[Node]> = gen_root.chain(gen_headers).chain(gen_nodes).collect();

                // For the nodes, the row, col fields are accurate.
                //  We do still have to fill in the neighbour idc.

                // The first num_cols + 1 entries are headers.
                for c in 0..(num_cols + 1) {
                        let l_idx = c;
                        let r_idx = l_idx + 1;
                        nodes[l_idx].r = r_idx as NodeIdx;
                        nodes[r_idx].l = l_idx as NodeIdx;
                }
                nodes[0].l = num_cols as NodeIdx;
                nodes[num_cols].r = 0;

                let n_all = nodes.len();

                // Gives iterator over indices of normal nodes, not headers.
                let normal_node_it = || (num_cols + 1)..n_all;

                // Now each row of the nodes.
                for r in 0..num_rows {
                        let r = r as NodeIdx;

                        // We find the first node in the row.
                        let mut idx_it = normal_node_it();
                        let Some(first_idx) = idx_it.find(|&i| nodes[i].row == r) else {
                                panic!("Empty row given");
                        };

                        let mut last_idx = first_idx;
                        for next_idx in idx_it {
                                if nodes[next_idx].row != r {
                                        continue;
                                }
                                nodes[next_idx].l = last_idx as NodeIdx;
                                nodes[last_idx].r = next_idx as NodeIdx;
                                last_idx = next_idx;
                        }

                        // cycle
                        nodes[last_idx].r = first_idx as NodeIdx;
                        nodes[first_idx].l = last_idx as NodeIdx;
                }

                // And columns are almost the same.
                for c in 0..num_cols {
                        let c = c as NodeIdx;

                        // We find the first node in the col.
                        let mut idx_it = normal_node_it();
                        let Some(first_idx) = idx_it.find(|&i| nodes[i].col == c) else {
                                panic!("Empty column given");
                        };

                        let mut last_idx = first_idx;
                        for next_idx in idx_it {
                                if nodes[next_idx].col != c {
                                        continue;
                                }
                                nodes[next_idx].u = last_idx as NodeIdx;
                                nodes[last_idx].d = next_idx as NodeIdx;
                                last_idx = next_idx;
                        }

                        // cycle with headers between.
                        let h_idx = (c + 1) as usize;
                        nodes[last_idx].d = h_idx as NodeIdx;
                        nodes[first_idx].u = h_idx as NodeIdx;
                        nodes[h_idx].d = first_idx as NodeIdx;
                        nodes[h_idx].u = last_idx as NodeIdx;
                }

                let mut sizes: Box<[u64]> = std::iter::repeat_n(0, num_cols).collect();
                for i in normal_node_it() {
                        let col = nodes[i].col;
                        sizes[col as usize] += 1;
                }

                // root is largely inititialized, but that's ok.
                DancingLinkArray {nodes, sizes}
        }

        pub
        fn construct_from_idc <I: IntoIterator<Item = (usize, usize)>> (elems: I) -> Option<DancingLinkArray>
        {
                let mut idc_arr: Box<[(usize, usize)]> = elems.into_iter().collect();

                // We sort this array row major.
                fn ord ((r1, c1): &(usize, usize), (r2, c2): &(usize, usize)) -> Ordering
                {
                        let r_comp = r1.cmp(r2);
                        if r_comp.is_eq() {
                                r_comp
                        } else {
                                c1.cmp(c2)
                        }
                }
                idc_arr.sort_unstable_by(ord);
                // Now we test for equality between elements.
                // If so, we return nothing.

                let opt_maxrow_col = idc_arr.iter().cloned()
                        .reduce(|(r1, c1), (r2, c2)|
                                (std::cmp::max(r1, r2), std::cmp::max(c1, c2)));

                let (num_rows, num_cols) = opt_maxrow_col
                    .map(|(m_r, m_c)| (m_r + 1, m_c + 1))
                        .unwrap_or((0, 0));

                if idc_arr.iter().all_unique() {
                        let dla = Self::construct_from_sorted_unsafe(idc_arr.into_iter(), num_rows, num_cols);
                        Some (dla)
                } else {
                        None
                }
        }

        // Fills in array such that array[r] is a NodeIndex to a node in row r.
        pub
        fn to_each_row (&self, output: &mut Vec<NodeIdx>)
        {
                // By construction, Each row should have at least one node.
                assert!(output.is_empty());
                let n_rows = self.num_rows();
                output.reserve(n_rows);
                let mut next_row = 0;
                for idx in self.get_it_over_node_idc() {
                        if self.get_row(idx) == next_row {
                                output.push(idx);
                                next_row += 1;
                        }
                }
                debug_assert_eq!(output.len(), n_rows);
        }
}
