mod dancing_link_array;
use dancing_link_array as dla;
use itertools::Itertools;

pub
struct UCSolver <'a, R: Eq> {
        array: dla::DancingLinkArray,

        row_dat: &'a [R],

        // Keeps track of the state we put the array in since construction.
        rm_rows: Vec<usize>,

        // We remember a node for each row.
        to_rows: Box<[dla::NodeIdx]>
}


// Todo: implement more constructors.
impl <'a, R: Eq> UCSolver <'a, R> {

        pub
        fn from_pred <C, P: Fn(&R, &C) -> bool> (rows: &'a [R], cols: &[C], p: P) -> UCSolver <'a, R>
        {
                let row_dat = rows;

                let idc_gen = rows.iter().enumerate()
                                .cartesian_product(cols.iter().enumerate())
                        .filter (|((_, r), (_, c))| p(r, c))
                        .map (|((r_idx, _), (c_idx, _))| (r_idx, c_idx));

                let dla = dla::DancingLinkArray::construct_from_sorted_unsafe(idc_gen, rows.len(), cols.len());
                let mut to_rows: Vec<dla::NodeIdx> = Vec::new();
                dla.to_each_row(&mut to_rows);

                UCSolver {array: dla, row_dat, rm_rows: Vec::new(), to_rows: to_rows.into_boxed_slice()}
        }

        pub
        fn set_state <I: IntoIterator<Item = &'a R>>(&mut self, r_it: I)
        {
                for rref in r_it {
                        let opt_r_idx = self.row_dat.iter().position(|row| *row == *rref);
                        let Some(r_idx) = opt_r_idx else {
                                panic! ("Tried to remove a non-existant row!");
                        };
                        if self.rm_rows.contains(&r_idx) {
                                panic!("Tried to remove an already removed row!");
                        }
                        let node_entry: dla::NodeIdx = self.to_rows[r_idx];

                        self.array.rm_row(node_entry);
                        self.rm_rows.push(r_idx);
                }

        }

        pub
        fn recover_n (&mut self, n: usize)
        {
                for _ in 0..n {
                        let r_idx = self.rm_rows.pop().expect("Tried to recover further than initial state!");
                        let entry_node = self.to_rows[r_idx];
                        self.array.insert_row(entry_node);
                }
        }

        pub
        fn solve_one (&mut self) -> Option<Vec<&'a R>>
        {
                let idc = self.array.solve_one()?;
                Some (idc.into_iter().map(|idx| &self.row_dat[idx as usize]).collect())
        }
        pub
        fn solve_many (&mut self) -> Vec<Vec<&'a R>>
        {
                let sols = self.array.solve_one();
                sols.into_iter().map(|sol| {
                        sol.into_iter().map(|idx| &self.row_dat[idx as usize]).collect()
                }).collect()
        }
}
