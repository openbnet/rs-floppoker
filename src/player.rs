use rs_handstrength::Card;
use std::default::Default;

#[derive(Debug, Clone)]
pub struct Player {
    pub seat: u8,
    pub chips: u16,
    pub is_all_in: bool,
    pub is_folded: bool,
    pub hand: [Card; 4],
}

impl Player {


    pub fn new(seat_number: u8, chips: u16) -> Self {
        Player {
            seat: seat_number,
            chips,
            is_all_in: false,
            is_folded: false,
            hand: [Card::default(); 4],
        }
    }


    pub fn add_chips(&mut self, &amt: &u16) {
        self.chips += amt;
    }

    pub fn remove_chips(&mut self, &amt: &u16) {
        if self.chips < amt {
            panic!("player not enough chips");
        } else {
            self.chips -= amt;
        }
    }

    pub fn new_hand(&mut self, cards: [Card; 4]) {
        self.hand = cards;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_handstrength::Suit;
    #[test]
    fn test_new() {
        let player = Player::new(1, 100);
        assert_eq!(player.seat, 1);
        assert_eq!(player.chips, 100);
        assert_eq!(player.is_all_in, false);
        assert_eq!(player.is_folded, false);
        assert_eq!(player.hand, [Card::default(); 4]);
    }

 

    #[test]
    fn test_add_chips() {
        let mut player = Player::new(1, 100);
        player.add_chips(&50);
        assert_eq!(player.chips, 150);
    }

    #[test]
    fn test_remove_chips() {
        let mut player = Player::new(1, 100);
        player.remove_chips(&50);
        assert_eq!(player.chips, 50);
    }

    #[test]
    #[should_panic(expected = "player not enough chips")]
    fn test_remove_chips_not_enough_chips() {
        let mut player = Player::new(1, 100);
        player.remove_chips(&150);
    }

    #[test]
    fn test_new_hand() {
        let mut player = Player::new(1, 100);
        let cards = [
            Card { value: 1, suit: Suit::S },
            Card { value: 13, suit: Suit::H },
            Card { value: 12, suit: Suit::D },
            Card { value: 11, suit: Suit::C },
        ];
        player.new_hand(cards);
        assert_eq!(player.hand, cards);
    }

        #[test]
        fn test_get_hand() {
            let mut player = Player::new(1, 100);
            let cards = [
                Card { value: 1, suit: Suit::S },
                Card { value: 13, suit: Suit::H },
                Card { value: 12, suit: Suit::D },
                Card { value: 11, suit: Suit::C },
            ];
            player.new_hand(cards);
            assert_eq!(&player.hand, &cards);
        }
    }
