use wordle_solver::*;

use clap::Parser;

const WORD_LEN: usize = 5;

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value_t = num_cpus::get())]
    threads: usize,

    inputs: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let mut inputs = vec![];

    for input in args.inputs.iter() {
        let sep = match input.find(':') {
            Some(x) => x,
            None => panic!("unknown input '{}' (expected <guess>:<response>, see -h for info)", input),
        };
        let guess = &input[..sep];
        let response: Vec<_> = input[sep+1..].chars().map(|ch| match ch {
            'c' => Hint::Correct,
            'p' => Hint::Present,
            'a' => Hint::Absent,
            x => panic!("unknown response '{}' (expected 'c' (correct), 'p' (present), or 'a' (absent))", x),
        }).collect();
        inputs.push((guess, response));
    }

    let dictionary = Dictionary::with_words(WORD_LEN, include_str!("guess-list.txt").split_whitespace()).unwrap();

    let mut puzzle = Puzzle::new(&dictionary);
    for (guess, response) in inputs.iter() {
        puzzle.guess(guess, response).unwrap();
    }

    println!("input summary:\n{}", puzzle);
    let (best_guess, worst_rem, avg_rem) = puzzle.best_guess(args.threads).unwrap();
    println!("best guess: {}\nremaining words: {} worst, {} avg.", best_guess, worst_rem, avg_rem);
}
