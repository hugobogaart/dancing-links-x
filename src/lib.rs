mod dlx;
// mod DancingLinkArray;

pub mod dlx2;

// Public interface to the DLX array.

// todo: make_choices_ref lifetimes don't make sense
// The liftims of the given rows should not be bound to that of the solver.


pub
struct DLXSolver <'a, R: Eq> {
        arr_intern: dlx::DancingLinkArray<'a, R>
}


impl <'a, R: Eq> DLXSolver <'a, R> {

        pub fn from_iter_general<I, OR, OC, FRef, FR, FC> (it: I, iref: FRef, i2r: FR, i2c: FC) -> Option<Self>
        where I: Iterator,
        OR: Ord,
        OC: Ord,
        FRef: Fn (&I::Item) -> &'a R,
        FR: Fn(&I::Item) -> OR,
        FC: Fn(&I::Item) -> OC
        {
                let opt_intern = dlx::DancingLinkArray::from_iter_general(it, iref, i2r, i2c);
                opt_intern.map(|arr_intern| DLXSolver {arr_intern})
        }

        pub fn from_pred <C, Pred> (rows: &'a [R], cols: &[C], p: Pred) -> Option<Self>
        where
                Pred: Fn (&R, &C) -> bool,
        {
                let opt_intern = dlx::DancingLinkArray::from_pred(rows, cols, p);
                opt_intern.map(|arr_intern| DLXSolver {arr_intern})
        }

        pub fn make_choices <I: Iterator<Item=&'a R>> (&mut self, choices: I)
        {
                self.arr_intern.make_choices_ref(choices);
        }

        pub fn unmake_choices (&mut self, n_choices: usize)
        {
                self.arr_intern.unmake_choices(n_choices);
        }

        pub fn solve_one (&mut self) -> Option <Vec<&'a R>>
        {
                self.arr_intern.solve_one_ref()
        }
        pub fn solve_many (&mut self) -> Vec <Vec<&'a R>>
        {
                self.arr_intern.solve_many_ref()
        }
        pub fn solve_one_with <I: IntoIterator<Item=&'a R>> (&mut self, choices: I) -> Option <Vec<&'a R>>
        {
                let res = self.arr_intern.solve_one_ref();
                None
        }

        pub fn solve_many_with <I: IntoIterator<Item=&'a R>> (&mut self, choices: I) -> Vec <Vec<&'a R>>
        {
                self.arr_intern.solve_many_ref()
        }

}
