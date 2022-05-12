use std::sync::Mutex;
use clap::Parser;
use wordle_solver::*;

const WORD_LEN: usize = 5;

#[derive(Parser)]
enum Args {
    /// Solve a wordle puzzle by predicting the best guess to make next
    Solve {
        #[clap(short, long, default_value_t = num_cpus::get())]
        threads: usize,

        inputs: Vec<String>,
    },
    /// Benchmark the performance of the solver on all possible 5-letter english words
    /// (includes words not used as answers by wordle itself)
    Bench {
        #[clap(short, long, default_value_t = num_cpus::get())]
        threads: usize,
        /// Also output the number of guesses needed for each tested word
        /// (a consistent ordering of words in the output is not guaranteed)
        #[clap(short, long)]
        verbose: bool,
    },
}

fn main() {
    let args = Args::parse();
    let raw_words = include_str!("guess-list.txt").split_whitespace();
    let dictionary = Dictionary::with_words(WORD_LEN, raw_words.clone()).unwrap();

    match args {
        Args::Solve { threads, inputs } => {
            let mut parsed_inputs = vec![];

            for input in inputs.iter() {
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
                parsed_inputs.push((guess, response));
            }

            let mut puzzle = Puzzle::new(&dictionary);
            for (guess, response) in parsed_inputs.iter() {
                puzzle.guess(guess, response).unwrap();
            }

            println!("input summary:\n{}", puzzle);
            let (best_guess, worst_rem, avg_rem) = puzzle.best_guess(threads).unwrap();
            println!("best guess: {}\nremaining words: {} worst, {} avg.", best_guess, worst_rem, avg_rem);
        }
        Args::Bench { mut threads, verbose } => {
            threads = threads.max(1);

            let init_guess = Puzzle::new(&dictionary).best_guess(threads).unwrap().0;
            let words_iter = Mutex::new(raw_words.into_iter().fuse());
            let results = Mutex::new(vec![]);

            crossbeam::scope(|s| {
                for _ in 0..threads {
                    s.spawn(|_| {
                        loop {
                            let answer = match words_iter.lock().unwrap().next() {
                                Some(x) => x,
                                None => break,
                            };
                            let mut puzzle = Puzzle::new(&dictionary);
                            let mut guesses = 0u8;

                            loop {
                                let guess = match guesses {
                                    0 => init_guess.clone(),
                                    _ => puzzle.best_guess(1).unwrap().0,
                                };
                                guesses += 1;
                                puzzle.guess(&guess, &get_hint(&guess, answer).unwrap()).unwrap();
                                if guess == answer { break }
                            }

                            results.lock().unwrap().push(guesses);
                            if verbose { println!("{} took {} guesses", answer, guesses); }
                        }
                    });
                }
            }).unwrap();

            if verbose { println!(); }
            let results = results.into_inner().unwrap();

            let mut min = u8::MAX;
            let mut max = 0;
            let mut avg = 0.0;
            for &x in results.iter() {
                min = min.min(x);
                max = max.max(x);
                avg += x as f64;
            }
            avg /= results.len() as f64;

            let mut std = 0.0;
            for &x in results.iter() {
                let diff = x as f64 - avg;
                std += diff * diff;
            }
            std /= results.len() as f64;
            std = std.sqrt();

            println!("results over {} words:", results.len());
            println!("min: {}", min);
            println!("max: {}", max);
            println!("avg: {:.04}", avg);
            println!("std: {:.04}", std);
        }
    }
}
