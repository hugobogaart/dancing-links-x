use dancing_links_x::dlx;
use itertools::Itertools;

#[derive(Clone, PartialEq, Eq, Debug)]
struct NBoard {
        n:      usize,
        board:  Box<[bool]>
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum NQueenConstraint {
        Horizontal (usize),     // strict
        Vertical (usize),       // strict

        // north-east and north-west diagonals.
        // For NE e identify the central diagonal from the bottom left with 0, +i the i-th bottom square
        // and -i with the i-th left square (from the bottom )
        // For NW the bottom right tile is 0, right squares + and bottom are -.
        DiagonalNE (i64),     // optional: Not required but at most one queen in the diagonal
        DiagonalNW (i64),     // optional: Not required but at most one queen in the diagonal
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct NQueenChoice {
        r: usize,
        c: usize,
}

impl NBoard {
        fn get_empty (n: usize) -> NBoard
        {
                NBoard {
                        n,
                        board: std::iter::repeat_n(false, n * n).collect()
                }
        }

        fn from_sol <I: IntoIterator<Item = (usize, usize)>> (n: usize, sol: I) -> NBoard
        {
                let mut board = Self::get_empty(n);
                sol.into_iter().for_each(|(r, c)| board.place_queen(r, c));
                board
        }
        fn from_choices <I: IntoIterator<Item = NQueenChoice>> (n: usize, sol: I) -> NBoard
        {
                Self::from_sol(n, sol.into_iter().map(|ch| (ch.r, ch.c)))
        }

        fn print (&self)
        {
                let mut out = String::new();
                for r in 0..self.n {
                        for c in 0..self.n {
                                let ch = if self.board[self.to_idx(r, c)] {'*'} else {'.'};
                                out.push(ch);
                                out.push(' ');
                        }
                        out.push('\n');
                }
                print!("{}", out);
        }

        fn to_idx (&self, r: usize, c: usize) -> usize
        {
                r * self.n + c
        }
        fn has_queen (&self, r: usize, c: usize) -> bool
        {
                self.board[self.to_idx(r, c)]
        }
        fn place_queen (&mut self, r: usize, c: usize)
        {
                assert!(!self.has_queen(r, c));
                self.board[self.to_idx(r, c)] = true
        }

        fn empty_strict_constraints (n: usize) -> Box<[NQueenConstraint]>
        {
                let hor_gen = (0..n).map(NQueenConstraint::Horizontal);
                let vert_gen = (0..n).map(NQueenConstraint::Vertical);
                hor_gen.chain(vert_gen).collect()
        }
        fn empty_optional_constraints (n: usize) -> Box<[NQueenConstraint]>
        {
                assert! (n > 0);
                // there are exactly 2 n - 1 per diagonal.
                let highest = (n - 1) as i64;
                let lowest = -highest;
                let ne_diag_gen = (lowest..=highest).map(NQueenConstraint::DiagonalNE);
                let nw_diag_gen = (lowest..=highest).map(NQueenConstraint::DiagonalNW);
                ne_diag_gen.chain(nw_diag_gen).collect()
        }

        fn all_choices (n: usize) -> Box<[NQueenChoice]>
        {
                (0..n).cartesian_product(0..n).map(|(r, c)| NQueenChoice{r, c}).collect()
        }


}

impl NQueenChoice {
        fn to_diag_ne (&self) -> i64
        {
                (self.c as i64) - (self.r as i64)
        }

        fn to_diag_nw (&self, n: usize) -> i64
        {
                let inv_c = (n as i64) - 1 - (self.c as i64);
                (self.r as i64) - inv_c
        }

        fn satisfies (&self, constraint: &NQueenConstraint, n: usize) -> bool
        {
                match *constraint {
                        NQueenConstraint::Horizontal(r) => r == self.r,
                        NQueenConstraint::Vertical  (c) => c == self.c,
                        NQueenConstraint::DiagonalNE(d) => d == self.to_diag_ne(),
                        NQueenConstraint::DiagonalNW(d) => d == self.to_diag_nw(n),
                }
        }
}


fn main ()
{
        let n = 69;
        let rows = NBoard::all_choices(n);
        let strict_cols = NBoard::empty_strict_constraints(n);
        let opt_cols = NBoard::empty_optional_constraints(n);

        let mut solver = dlx::UCSolver::from_pred_opt(rows.as_ref(), strict_cols.as_ref(), opt_cols.as_ref(), |ch, cst| {
                ch.satisfies(cst, n)
        });

        let o_sol = solver.solve_one();
        if let Some(sol) = o_sol {
                println!("Solution");
                let solved_board = NBoard::from_choices(n, sol);
                solved_board.print();
        } else {
                println!("No solution");
        }
}
