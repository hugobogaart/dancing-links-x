mod dlx;

// Public interface to the DLX array.

pub
struct DancingLinkArray<'a, R: Eq> {
        arr_intern: dlx::DancingLinkArray<'a, R>
}


impl <'a, R: Eq> DancingLinkArray<'a, R> {

        pub fn from_iter_general<I, OR, OC, FRef, FR, FC> (it: I, iref: FRef, i2r: FR, i2c: FC) -> Option<Self>
        where I: Iterator,
        OR: Ord,
        OC: Ord,
        FRef: Fn (&I::Item) -> &'a R,
        FR: Fn(&I::Item) -> OR,
        FC: Fn(&I::Item) -> OC
        {
                let opt_intern = dlx::DancingLinkArray::from_iter_general(it, iref, i2r, i2c);
                opt_intern.map(|arr_intern| DancingLinkArray {arr_intern})
        }

        pub fn from_pred <C, Pred> (rows: &'a [R], cols: &[C], p: Pred) -> Option<Self>
        where
                Pred: Fn (&R, &C) -> bool,
        {
                let opt_intern = dlx::DancingLinkArray::from_pred(rows, cols, p);
                opt_intern.map(|arr_intern| DancingLinkArray {arr_intern})
        }

        pub fn solve_one_ref (&mut self) -> Option <Vec<&'a R>>
        {
                self.arr_intern.solve_one_ref()
        }
        pub fn solve_many_ref (&mut self) -> Vec <Vec<&'a R>>
        {
                self.arr_intern.solve_many_ref()
        }

        pub fn make_choices_ref <I: Iterator<Item=&'a R>> (&mut self, rows: I)
        {
                self.arr_intern.make_choices_ref(rows);
        }

        pub fn unmake_choices (&mut self, n_choices: usize)
        {
                self.arr_intern.unmake_choices(n_choices);
        }


}
