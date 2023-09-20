use serde::{Deserialize, Serialize};

pub use serde;
pub use serde_json;
pub use serde_with;

const NONCE_MIN: u32 = 0;
const NONCE_MAX: u32 = 0xFFFFFFFF;
const NONCE_SET_SPACE: u32 = NONCE_MAX - NONCE_MIN; 

/// The target block that players are trying to guess the nonce for.
#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct Target {
    pub block: u32,
    pub nonce: Option<u32>,
}

/// A players guess for a target block nonce.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Guess {
    pub block: Option<u32>,
    pub name: String,
    pub nonce: u32,
}

/// Block data from mempool.space.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Block {
    pub id: String,
    pub height: u32,
    pub nonce: u32,
}
/*
This function calculates the chance each guess has of winning by dividing 
the NONCE_SET_SPACE into ranges determined by proximity
to each guess. The win probability for a uniformly distributed 
nonce is the ratio of a guess' range to the total set space     
*/
pub fn get_guess_probabilities(guesses: &mut [Guess]) -> Vec<(String, f32)> {
    sort_guesses_by_nonce(guesses);

    let mut result = Vec::new();
    let mut lower_bound = NONCE_MIN;

    let mut guess_iter = guesses.iter();
    if let Some(mut current_guess) = guess_iter.next() {
        while let Some(next_guess) = guess_iter.next() {
            let upper_bound = current_guess.nonce + ((next_guess.nonce - current_guess.nonce) / 2);
            result.push((current_guess.name.clone(), range_probability(lower_bound, upper_bound)));
            lower_bound = upper_bound + 1;
            current_guess = next_guess;
        }
        let upper_bound = NONCE_MAX;
        result.push((current_guess.name.clone(), range_probability(lower_bound, upper_bound)));
    }

    result
}

fn range_probability(lower: u32, upper: u32) -> f32 {
    ((upper - lower) as f32) / (NONCE_SET_SPACE as f32)
}

pub fn sort_guesses_by_target_diff(guesses: &mut [Guess], target_nonce: u32) {
    guesses.sort_by(|a, b| {
        let target_a = target_nonce.abs_diff(a.nonce);
        let target_b = target_nonce.abs_diff(b.nonce);
        target_a.cmp(&target_b)
    })
}

pub fn sort_guesses_by_nonce(guesses: &mut [Guess]) {
    guesses.sort_by_key(|g| g.nonce)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn probabilities_for_empty_guesses() {
        let mut input = vec![];
        let output = get_guess_probabilities(input.as_mut_slice());
        assert_eq!(output.len(), 0);
    }

    #[test]
    fn probabilities_for_one_guess() {
        let mut input = vec![
            Guess { name: String::from("b"), nonce: 0xBBBBBBBB, block: None }
        ];
        let output = get_guess_probabilities(input.as_mut_slice());
        assert_eq!(output[0], (String::from("b"), 1.0));
    }

    #[test]
    fn probabilities_for_guesses() {
        let mut input = vec![
            Guess { name: String::from("ben"), nonce: 0xDEADBEEF, block: None },
            Guess { name: String::from("carol"), nonce: 0xBEEFDEAD, block: None }
        ];
        let output = get_guess_probabilities(input.as_mut_slice());
        
        assert_eq!(output[0], (String::from("carol"), 0.80784315));
        assert_eq!(output[1], (String::from("ben"), 0.19215687));
    }

    #[test]
    fn probabilities_for_boundary_guesses() {
        let mut input = vec![
            Guess { name: String::from("Max"), nonce: NONCE_MAX, block: None },
            Guess { name: String::from("Min"), nonce: NONCE_MIN, block: None }
        ];
        let output = get_guess_probabilities(input.as_mut_slice());
        
        assert_eq!(output[0], (String::from("Min"), 0.5));
        assert_eq!(output[1], (String::from("Max"), 0.5));
    }
}

