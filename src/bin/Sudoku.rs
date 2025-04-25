use std::num::NonZero;
use std::path::{Path, PathBuf};
use dancing_links_x::dlx;
use dancing_links_x::{self};
use itertools::Itertools;
use std::time::{self, Duration};
use std::env;

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

#[derive(Clone, Copy, PartialEq, Eq)]
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



const B: &str = "   1 2    6     7   8   9  4       3 5   7   2   8   1  9   8 5 7     6    3 4   ";
const EB: &str = "2...7..3.3......6......198.1.4.6....................9...5...6.....754.2...3.8..7.";




fn open_sudokus (filepath:PathBuf, skip_first: bool) -> Result<Box<[SudokuBoard]>, String>
{
        let contents: Vec<u8> = fs::read(filepath).expect("Could not open file");

        let s: String = String::from_utf8(contents).unwrap();
        let mut lines_it = s.lines();
        if skip_first {
                lines_it.next();
        }
        let gen_board = |str| -> Result<SudokuBoard, String> {
                if let Some(b) = SudokuBoard::new_from_str(str, b'0') {
                        Ok(b)
                } else {
                        Err (format!("Failed to create a board from the line {str}"))
                }
        };

        // Now we read.
        let res: Result<Box<[SudokuBoard]>, String> = lines_it
                .map(gen_board)
                .collect();
        res
}

fn write_sudokus (boards: &[SudokuBoard], sols: &[SudokuBoard], fname: PathBuf)
{
        let num = sols.len();
        assert_eq!(num, sols.len());

        let mut res: Vec<u8> = num.to_string().into_bytes();

        // There are (num_sols + 1) newline characters,and each line is 2 * 81 + 1 character.
        res.reserve(num + 1 + num * (2 * 81 + 1));
        res.push(b'\n');
        let mut tmp_buff: [u8; 81] = [0; 81];
        for (board, sol) in boards.iter().zip(sols.iter()) {
                board.to_raw_u8(b'0', &mut tmp_buff);
                res.extend(tmp_buff.iter());
                res.push(b',');
                sol.to_raw_u8(b'0', &mut tmp_buff);
                res.extend(tmp_buff.iter());
                res.push(b'\n');
        }

        // write
        fs::write(fname, res).unwrap();
}

const HARDEST: &str = "800000000003600000070090200050007000000045700000100030001000068008500010090000400";

struct FileSolveJob {
        path_to_file: PathBuf,
        path_to_sols: PathBuf,
        ignore_first: bool
}

enum Job {
        SolveFile (FileSolveJob),
        SolveArg (String),
}

struct CLArguments {
        job: Job,
        time: bool,
}

fn parse_args () -> Result<CLArguments, String>
{

        // The -f flag means we take a file.
        // The -t flag means we time it.
        // The -i flag can be used together with f, and means we ignore the first line.

        // if the output is not given with the -f, the default is <input>-sols.txt
        fn token_is_flag (t: &String) -> bool {
                t.starts_with(|c| c == '-')
        }
        let tok_args: Vec<String> = std::env::args().skip(1).collect();
        let mut t = false;
        let mut f = false;
        let mut i = false;

        // The first tokens are flags.
        let mut input_idx = 0;
        for arg in &tok_args {
                if token_is_flag(arg) {
                        if arg.len() == 1 {
                                return Err(String::from("Empty flag passed!"));
                        }
                        input_idx += 1;
                        if arg.contains('f') {
                                f = true;
                        }
                        if arg.contains('t') {
                                t = true;
                        }
                        if arg.contains('i') {
                                i = true;
                        }
                } else {
                        break;
                }
        }

        if f {
                // There are either one or two following arguments.
                let Some(to_input_path_str) = tok_args.get(input_idx) else {
                        return Err(String::from("No input file given with the -f flag!"));
                };
                let output_path_str: String = if input_idx + 1 < tok_args.len() {
                        tok_args[input_idx + 1].clone()
                } else {
                        let mut cpy = to_input_path_str.clone();
                        cpy.push_str("-sols.txt");
                        cpy
                };

                let fs_job = FileSolveJob {
                        path_to_file: PathBuf::from(to_input_path_str),
                        path_to_sols: PathBuf::from(output_path_str),
                        ignore_first: i
                };
                let job = Job::SolveFile(fs_job);
                Ok(CLArguments { job, time: t })
        } else {
                if i {
                        return Err(String::from("Can't use -i without -f!"));
                }
                let job = Job::SolveArg(tok_args[input_idx].clone());
                Ok (CLArguments { job, time: t })
        }
}

fn exec_job (job: Job, time_it: bool)
{
        match job {
                Job::SolveArg(b_str) => solve_board (b_str, time_it),
                Job::SolveFile(fjob) => solve_file  (fjob, time_it),
        }
}

fn solve_board (b_str: String ,time_it: bool)
{
        let Some(board) = SudokuBoard::new_from_str(b_str.as_str(), b'0') else {
                println!("Can't make a board from {b_str}");
                return;
        };
        let rows: &[Choice] = EMPTY_POSSIBLE_CHOICES.as_ref();
        let cols: &[Constraint] = EMPTY_CONSTRAINTS.as_ref();
        let made_choices = all_current_choices(&board);
        let mut solver = dlx::UCSolver::from_pred(rows, cols, choice_satisies_constraint);
        let start = std::time::Instant::now();
        let Some(sol) = solver.solve_one_with(&made_choices) else {
                println!("Found no solution!");
                return;
        };
        let dur = start.elapsed();
        board.print(' ');
        let solved_board = {
                let mut cpy = board.clone();
                cpy.make_moves(&sol);
                cpy
        };
        solved_board.print(' ');
        if time_it {
                let s = dur.as_secs_f64();
                println!("That took {} ms", s * 1000.0);
        }
}

fn solve_file (fjob: FileSolveJob ,time_it: bool)
{
        let sudokus;
        match open_sudokus(fjob.path_to_file, fjob.ignore_first) {
                Ok(s) => sudokus = s,
                Err(str) => {
                        println!("{}", str);
                        return
                }
        }

        let rows: &[Choice] = EMPTY_POSSIBLE_CHOICES.as_ref();
        let cols: &[Constraint] = EMPTY_CONSTRAINTS.as_ref();
        let mut solver = dlx::UCSolver::from_pred(rows, cols, choice_satisies_constraint);
        let all_made_choices: Vec<Vec<Choice>> = sudokus.iter().map(all_current_choices).collect();
        let mut sols: Vec<SudokuBoard> = Vec::with_capacity(sudokus.len());
        let all_made_choices: Vec<Vec<Choice>> = sudokus.iter().map(all_current_choices).collect();

        let t_start =  std::time::Instant::now();

        for (board, made_choices) in sudokus.iter().zip(all_made_choices.iter()) {
                let Some(sol) = solver.solve_one_with(made_choices) else {
                        println!("Could not solve board: ");
                        board.print(' ');
                        return;
                };
                let mut solved_board = board.clone();
                solved_board.make_moves(&sol);
                sols.push(solved_board);
        }
        let dur = t_start.elapsed();
        write_sudokus(sudokus.as_ref(), sols.as_slice(), fjob.path_to_sols);
        if time_it {
                let s = dur.as_secs_f64();
                println!("That took {} ms", s * 1000.0);
        }
}

fn main ()
{
        match parse_args() {
                Ok(cla)  => exec_job(cla.job, cla.time),
                Err(msg) => println!("{msg}"),
        }
}
