use wordle_solver::*;

const WORD_LEN: usize = 5;

// fn prompt_guess() -> String {
//     let mut res = String::new();
//     loop {
//         print!("Next guess: ");
//         res.clear();
//         std::io::stdin().read_line(&mut res).unwrap();

//         let clean = res.trim();
//         if WORD_LIST.contains(clean) { return clean.to_owned() }

//         if clean.len() != WORD_LEN { println!("'{}' is not a 5-letter word!", clean); }
//         else { println!("'{}' is not a known word!", clean); }
//     }
// }

fn main() {
    let mut puzzle = Puzzle::new(WORD_LEN);

    puzzle.guess("cares", &[Hint::Absent, Hint::Correct, Hint::Absent, Hint::Present, Hint::Present]).unwrap();
    puzzle.guess("satay", &[Hint::Present, Hint::Correct, Hint::Absent, Hint::Absent, Hint::Absent]).unwrap();
    puzzle.guess("bonie", &[Hint::Absent, Hint::Absent, Hint::Absent, Hint::Absent, Hint::Correct]).unwrap();
    puzzle.guess("douse", &[Hint::Absent, Hint::Absent, Hint::Correct, Hint::Correct, Hint::Correct]).unwrap();
    puzzle.guess("hause", &[Hint::Absent, Hint::Correct, Hint::Correct, Hint::Correct, Hint::Correct]).unwrap();




    println!("{}", puzzle);
    let (best_guess, worst_rem, avg_rem) = puzzle.best_guess(8).unwrap();
    println!("best guess: {} (worst rem: {}, avg rem: {})", best_guess, worst_rem, avg_rem);
}
