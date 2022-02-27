use wordle_solver::*;

const WORD_LEN: usize = 5;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut guesses = vec![];

    for arg in &args[1..] {
        if ["-h", "--help"].contains(&arg.as_str()) {
            println!("usage: {} [<guess>:<response>...]\n    <response> is a sequence of 'c' (correct), 'p' (present), and 'a' (absent)", args[0]);
            std::process::exit(0);
        }

        let sep = match arg.find(':') {
            Some(x) => x,
            None => panic!("unknown input '{}' (expected <guess>:<response>, see -h for info)", arg),
        };
        let guess = &arg[..sep];
        let response: Vec<_> = arg[sep+1..].chars().map(|ch| match ch {
            'c' => Hint::Correct,
            'p' => Hint::Present,
            'a' => Hint::Absent,
            x => panic!("unknown response '{}' (expected 'c' (correct), 'p' (present), or 'a' (absent))", x),
        }).collect();
        guesses.push((guess, response));
    }

    let mut puzzle = Puzzle::new(WORD_LEN);
    for (guess, response) in guesses.iter() {
        puzzle.guess(guess, response).unwrap();
    }

    println!("input summary:\n{}", puzzle);
    let (best_guess, worst_rem, avg_rem) = puzzle.best_guess(8).unwrap();
    println!("best guess: {}\nremaining words: {} worst, {} avg.", best_guess, worst_rem, avg_rem);
}
