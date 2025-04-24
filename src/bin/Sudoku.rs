use std::num::NonZero;
use std::path::Path;
use dancing_links_x::dlx2;
use dancing_links_x::{self, DLXSolver};
use std::time;

use std::fs;
use std::path;

// For initializing some globals.
use std::sync::LazyLock;


type RowT = u8;
type ColT = u8;
type BoxT = u8;
type ExtraBoxT = u8;
type ValT = u8;


// possibly 1..=9
#[derive(Clone, Copy, Debug)]
struct SudokuValue (Option<NonZero<ValT>>);

const EMPTY_VAL: SudokuValue = SudokuValue(None);

#[derive(Clone)]
struct SudokuBoard {
        board: [SudokuValue; 9 * 9]
}

static EMPTY_BOARD: SudokuBoard = SudokuBoard {board: [EMPTY_VAL; 9 * 9]};


// A choice contains of a position and a value 1-9
#[derive(Clone, Copy, PartialEq, Eq)]
struct Choice {
        row: RowT,
        col: ColT,
        val: ValT,
}

#[derive(Clone, Copy)]
enum Constraint {
        Row (RowT, ValT),
        Column (ColT, ValT),
        Box (BoxT, ValT),
        Position (RowT, ColT)
}

#[derive(Clone, Copy)]
enum ConstraintExtra {
        Normal (Constraint),
        ExtraBox(ExtraBoxT, ValT)
}

fn gen_all_choices () -> Vec<Choice>
{
        let mut choices: Vec<Choice> = Vec::new();
        for r in  1..=9 {
                for c in  1..=9 {
                        for v in  1..=9 {
                                choices.push(Choice {row: r, col: c, val: v});
                        }
                }
        }
        choices
}

static EMPTY_POSSIBLE_CHOICES: LazyLock<Box<[Choice]>> = LazyLock::new(|| {
        gen_all_choices().into_boxed_slice()
});

fn all_current_choices (board: &SudokuBoard) -> Vec<Choice>
{
        board.board.iter()
            .enumerate()
            .filter_map(|(i, SudokuValue (v))| {
                    let Some(val) = v else {
                            return None;
                    };
                    let row = ((i / 9) + 1).try_into().unwrap();
                    let col = ((i % 9) + 1).try_into().unwrap();
                    Some (Choice {row, col, val: val.get()})
            }).collect()
}

fn all_constraints () -> Vec<Constraint>
{
        let mut v: Vec<Constraint> = Vec::new();
        for i in 1..=9 {
                for j in 1..=9 {
                        v.push(Constraint::Row(i, j));
                        v.push(Constraint::Column(i, j));
                        v.push(Constraint::Box(i, j));
                        v.push(Constraint::Position(i, j));
                }
        }
        v
}

static EMPTY_CONSTRAINTS: LazyLock<Box<[Constraint]>> = LazyLock::new(|| {
        all_constraints().into_boxed_slice()
});


fn get_all_constraints_extra () -> Vec<ConstraintExtra>
{
        let v: Vec<Constraint> = all_constraints();
        let mut ev: Vec<ConstraintExtra> = v.into_iter().map(ConstraintExtra::Normal).collect();
        for eb in 1..=4 {
                for v in 1..=9 {
                        ev.push(ConstraintExtra::ExtraBox(eb, v));
                }
        }
        ev
}
static EMPTY_CONSTRAINTS_EXTRA: LazyLock<Box<[ConstraintExtra]>> = LazyLock::new(|| {
        get_all_constraints_extra().into_boxed_slice()
});


fn to_box_nr (row: u8, col: u8) -> BoxT
{
        // row, col are 1-indexed.
        let r = row - 1;
        let c = col - 1;
        let b = ((r / 3) * 3) + (c / 3);
        b + 1
}

fn to_extra_box_nr (row: u8, col: u8) -> Option<ExtraBoxT>
{
        if (2..=4).contains(&row) {
                if (2..=4).contains(&col) {
                        return Some(1);
                } else if (6..=8).contains(&col) {
                        return Some(2);
                }
        } else if (6..=8).contains(&row) {
                if (2..=4).contains(&col) {
                        return Some(3);
                } else if (6..=8).contains(&col) {
                        return Some(4);
                }
        }

        None
}



fn choice_satisies_constraint (choice: &Choice, constraint: &Constraint) -> bool
{
        let Choice {row: cr, col: cc, val: cv} = *choice;

        match *constraint {
                Constraint::Row(r, v) => cr == r && cv == v,
                Constraint::Column(c, v) => cc == c && cv == v,
                Constraint::Box(b, v) => cv == v && to_box_nr(cr, cc) == b,
                Constraint::Position(r, c) => cr == r && cc == c,
        }
}



fn choice_satisies_extra_constraint (choice: &Choice, extra: &ConstraintExtra) -> bool
{
        match *extra {
                ConstraintExtra::Normal (norm) => choice_satisies_constraint(choice, &norm),
                ConstraintExtra::ExtraBox(et, ev) => choice.val == ev && {
                        let Choice {row: r, col: c, ..} = *choice;
                        if let Some(t) = to_extra_box_nr(r, c) {
                                t == et
                        } else {
                                false
                        }
                }
        }
}


impl SudokuValue {
        fn to_char (&self, empty: char) -> char
        {
                match self.0 {
                        None => empty,
                        Some (c) => char::from_digit (c.get() as u32, 10).unwrap()
                }
        }
}



impl SudokuBoard {

        // We 1-index.
        fn index (&self, row: RowT, col: ColT) -> &SudokuValue
        {
                if row >= 1 && row <= 9 && col >= 1 && col <= 9 {
                        let idx: usize = usize::from((row - 1) * 9 + col - 1);
                        &self.board[idx]
                } else {
                        panic! ("Tried to index sudokuboard out of bounds");
                }
        }

        fn to_str (&self, empty: char) -> String
        {
                let mut str = String::new();
                for r in 1..=9 {
                        for c in 1..=9 {
                                str.push(self.index(r, c).to_char(empty));
                                str.push(' ');
                        }
                        str.push('\n');
                }
                str
        }

        fn print (&self, empty: char)
        {
                print!("{}", self.to_str(empty));
        }

        fn new_from_str (s: &str, empty: u8) -> Option<SudokuBoard>
        {
                if s.len() != 9 * 9 {
                        return None;
                }

                // A u8 can be a space, in which case we have the empty value,
                // A char '1' - '9', in which case we have a valid value.
                // Other input is a user error.
                let conv = |c| -> Option<SudokuValue>{
                        if c == empty {
                                Some(SudokuValue (None))
                        } else {
                                NonZero::new(c - b'0').map(|v| SudokuValue (Some (v)))
                        }
                };

                let vals: Vec<SudokuValue> = s.bytes().map(conv).collect::<Option<Vec<SudokuValue>>>()?;


                let board: [SudokuValue; 9 * 9] = match vals.try_into() {
                        Ok(a)   => a,
                        _       => panic!("\"vals\" vector not {} elements", 9 * 9)
                };

                // We already enforced the right size.
                Some (SudokuBoard {board})
        }

        fn make_move (&mut self, mv: &Choice)
        {
                let r = usize::from(mv.row - 1);
                let c = usize::from(mv.col - 1);
                let v = SudokuValue(NonZero::try_from(mv.val).ok() );
                self.board[r * 9 + c] = v;
        }

        fn make_moves (&mut self, mvs: &[Choice])
        {
                mvs.iter().for_each(|mv| self.make_move(mv));
        }
        fn make_moves_ref (&mut self, mvs: &[&Choice])
        {
                mvs.iter().for_each(|mv| self.make_move(*mv));
        }

        // returns as linear string
        fn to_raw_u8 (&self, empty: u8, into: &mut [u8; 81])
        {
                for i in 0..81 {
                        into[i] = match self.board[i] {
                                SudokuValue(None) => empty,
                                SudokuValue(Some(v)) => v.get() + b'0',
                        };
                }
        }
}

use std::hint;

fn benchmark <F: FnOnce ()> (f: F) -> std::time::Duration
{
        let start = std::time::Instant::now();
        f();
        start.elapsed()
}


fn test_speed (board: &SudokuBoard, n: usize) -> std::time::Duration
{
        // Solver constructed
        let rows: &[Choice] = EMPTY_POSSIBLE_CHOICES.as_ref();
        let cols: &[Constraint] = EMPTY_CONSTRAINTS.as_ref();
        let made_choices = all_current_choices(board);
        let mut solver = DLXSolver::from_pred(rows, cols, choice_satisies_constraint).unwrap();
        solver.make_choices(made_choices.iter());

        let run = || {
                for _ in 0..n {
                        std::hint::black_box(solver.solve_one());
                }
        };
        benchmark(run)
}

const B: &str = "   1 2    6     7   8   9  4       3 5   7   2   8   1  9   8 5 7     6    3 4   ";
const EB: &str = "2...7..3.3......6......198.1.4.6....................9...5...6.....754.2...3.8..7.";

fn solve_one_extra_board_once (board: &SudokuBoard) -> Option<SudokuBoard>
{
        let rows: &[Choice] = EMPTY_POSSIBLE_CHOICES.as_ref();
        let cols: &[ConstraintExtra] = EMPTY_CONSTRAINTS_EXTRA.as_ref();
        let made_choices = all_current_choices(&board);

        /*
        let mut solver = DLXSolver::from_pred(rows, cols, choice_satisies_extra_constraint).expect("This should be valid input");
        solver.make_choices(made_choices.iter());
        */
        let mut solver = dlx2::UCSolver::from_pred(rows, cols, choice_satisies_extra_constraint);
        solver.set_state(made_choices.iter());

        let sol = solver.solve_one()?;
        let solved_board = {
                let mut clone = board.clone();
                clone.make_moves_ref(sol.as_slice());
                clone
        };
        Some(solved_board)
}


fn open_sudokus () -> Vec<SudokuBoard>
{
        let to_sudokus = Path::new("./sudoku-files/all_17_clue_sudokus.txt");
        assert!(to_sudokus.is_file());
        let contents: Vec<u8> = fs::read(to_sudokus).expect("Could not open file");

        let s: String = String::from_utf8(contents).unwrap();
        let mut lines_it = s.lines();
        let first_line = lines_it.next().unwrap();

        // Now we read.
        let num_sudokus: usize = first_line.parse().unwrap();
        lines_it.map(|str| SudokuBoard::new_from_str(str, b'0').unwrap()).collect()
}

fn write_sudokus (boards: &[SudokuBoard], sols: &[SudokuBoard])
{
        let num = sols.len();
        assert_eq!(num, sols.len());

        let to_sudokus = Path::new("./sudoku-files/all_17_clue_sudokus_sols.txt");

        let mut res: Vec<u8> = num.to_string().into_bytes();

        // There are (num_sols + 1) newline characters,and each line is 2 * 81 + 1 character.
        res.reserve(num + 1 + num * (2 * 81 + 1));
        res.push(b'\n');
        let mut buff: [u8; 81] = [0; 81];
        for (board, sol) in boards.iter().zip(sols.iter()) {
                board.to_raw_u8(b'0', &mut buff);
                res.extend(buff.iter());
                res.push(b',');
                sol.to_raw_u8(b'0', &mut buff);
                res.extend(buff.iter());
                res.push(b'\n');
        }

        // write
        fs::write(to_sudokus, res).unwrap();
}

fn solve (boards: &[SudokuBoard]) -> Vec<SudokuBoard>
{
        let rows: &[Choice] = EMPTY_POSSIBLE_CHOICES.as_ref();
        let cols: &[Constraint] = EMPTY_CONSTRAINTS.as_ref();
        // let mut solver = DLXSolver::from_pred(rows, cols, choice_satisies_constraint).expect("This should be valid input");
        let mut solver = dlx2::UCSolver::from_pred(rows, cols, choice_satisies_constraint);

        let mut sols: Vec<SudokuBoard> = Vec::new();
        sols.reserve_exact(boards.len());

        let all_made_choices: Vec<Vec<Choice>> = boards.iter().map(all_current_choices).collect();

        for (board, made_choices) in boards.iter().zip(all_made_choices.iter()) {

                let num_choices = made_choices.len();

                // solver.make_choices(made_choices.iter());
                solver.set_state(made_choices.iter());
                let optsol = solver.solve_one();
                // solver.unmake_choices(num_choices);
                solver.recover_n(num_choices);


                let mut solved_board = board.clone();
                solved_board.make_moves_ref(optsol.unwrap().as_slice());
                sols.push(solved_board);
        }

        sols
}


fn main ()
{

        let mut num_sudokus: u32 = 0;
        let time = benchmark(|| {
                let all_sudokus = open_sudokus();
                let all_solutions = solve (all_sudokus.as_slice());
                write_sudokus(all_sudokus.as_slice(), all_solutions.as_slice());
                num_sudokus = all_sudokus.len() as u32;
        });

        let nsecs = time.as_secs_f64();
        let nmillis_per_run = 1000.0 * nsecs / f64::from(num_sudokus);
        let nmicros_per_run = 1000.0 * nmillis_per_run;

        println!("Solving {num_sudokus} sudokus took {nsecs} seconds\nwhich is {nmillis_per_run} ms per solution\nwhich is {nmicros_per_run} us per run");

        return;

        let board = SudokuBoard::new_from_str(EB, b'.').unwrap();
        /*
        let n: u32 = 10_000;
        let speed = test_speed (&board, n as usize);
        let nsecs = speed.as_secs_f64();
        let nmillis_per_run = 1000.0 * nsecs / f64::from(n);
        let nmicros_per_run = 1000.0 * nmillis_per_run;

        println!("Solving {n} times took {nsecs} seconds\nwhich is about {nmillis_per_run} ms per solution\nwhich is {nmicros_per_run} us per run");

        // return;
        */



        board.print(' ');
        let opt_solved_board = solve_one_extra_board_once(&board);
        if let Some (solved_board) = &opt_solved_board {
                println!("Solution\n");
                solved_board.print(' ');
        } else {
                println!("no solution");
        }
}
