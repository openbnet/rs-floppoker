use rs_handstrength::{Card, Suit, sort_cards4, sort_cards3};
use rand::{seq::SliceRandom, SeedableRng, rngs::StdRng};

#[derive(Debug, Clone)]
pub struct Deck {
    cards: [Card; 52],
    card_index: u8,
    seed: u64,
}

impl Deck {
    // Constructor: Initializes a new deck with shuffled cards
    pub fn new(seed: u64) -> Deck {
        let mut cards = [Card { value: 0, suit: Suit::S }; 52]; // Temporary initialization
        let mut index = 0;

        for suit in &[Suit::S, Suit::H, Suit::C, Suit::D] {
            for value in 1..=13 {
                cards[index] = Card { value, suit: *suit };
                index += 1;
            }
        }

        let mut rng = StdRng::seed_from_u64(seed);
        cards.shuffle(&mut rng);

        Deck {
            cards,
            card_index: 0,
            seed,
        }
    }

    // Shuffles the deck
    pub fn shuffle(&mut self) {
        let mut rng = StdRng::seed_from_u64(self.seed);
        self.cards.shuffle(&mut rng);
        self.card_index = 0;
    }

    // func to draw 4 cards from the deck
    // take in self without num_cards and return [Card;4]
    // should increment the card_index by 4
    // should return the 4 drawn cards
    // should panic if there are not enough cards in the deck
    pub fn draw4(&mut self) -> [Card; 4] {
        if self.card_index + 4 > 52 {
            panic!("Not enough cards in the deck");
        }

        let cards = [0, 1, 2, 3]
            .map(|i| self.cards[(self.card_index + i ) as usize]);

        self.card_index += 4;
        sort_cards4(&cards)
    }
    pub fn draw3(&mut self) -> [Card; 3] {
        if self.card_index + 3 > 52 {
            panic!("Not enough cards in the deck");
        }

        let cards = [0, 1, 2]
            .map(|i| self.cards[(self.card_index + i ) as usize]);

        self.card_index += 3;
        sort_cards3(&cards)
    }
    pub fn draw1(&mut self) -> Card {
        if self.card_index + 1 > 52 {
            panic!("Not enough cards in the deck");
        }

        let card = self.cards[self.card_index as usize];
        self.card_index += 1;
        card
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_deck_has_52_cards() {
        let deck = Deck::new(42);
        assert_eq!(deck.cards.len(), 52);
    }

    #[test]
    fn test_shuffle_changes_order() {
        let deck1 = Deck::new(42);
        let mut deck2 = Deck::new(42);
        deck2.shuffle();

        assert_ne!(deck1.cards, deck2.cards);
    }
}

